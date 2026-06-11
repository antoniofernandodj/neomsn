# NeoMSN

Um app de mensagens instantâneas moderno inspirado no MSN Messenger, com protocolo binário próprio, suporte a desktop, mobile e web, e interface fiel ao espírito do MSN.

## Visão do Produto

- Interface visual inspirada no MSN Messenger (janelas de chat, lista de contatos com status, emoticons, etc.)
- Texto exibido em tempo real conforme o usuário digita — sem botão "Enviar"
- Botão **Concluir** finaliza a mensagem atual e gera um novo ID para a próxima
- Cada mensagem tem seu próprio UUID desde o início da digitação
- Suporte a salas (rooms), chats diretos (DMs) e presença de usuários (online/ausente/ocupado/invisível)
- Sincronização de histórico entre dispositivos
- Plataformas: **Desktop** (Linux/Windows/macOS), **Mobile** (Android/iOS), **Web** (WASM + WebSocket)

---

## Arquitetura Geral

O servidor é um único binário Rust com dois listeners independentes rodando em paralelo via `tokio::join!`. Eles compartilham o mesmo `Arc<AppState>` (pool de banco, sessões, presença). Não há detecção de protocolo por bytes — cada porta faz exatamente uma coisa.

```
porta 7777  →  TcpListener  →  NMP puro  (desktop, mobile)
porta 8080  →  axum (HTTP)  →  REST + WebSocket upgrade  (web, avatar upload)
```

```
┌──────────────────────────────────────────────────────────────┐
│                          CLIENTES                            │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐      ┌─────────────────┐  │
│  │   Desktop   │  │   Mobile    │      │       Web       │  │
│  │    (Iced)   │  │    (Iced)   │      │  (WASM + axum)  │  │
│  └──────┬──────┘  └──────┬──────┘      └────────┬────────┘  │
│         │  NMP/TCP :7777 │  NMP/TCP :7777        │ WS :8080  │
└─────────┼────────────────┼───────────────────────┼──────────┘
          │                │                       │
          │                │              ┌─────────▼──────────┐
          │                │              │  axum :8080        │
          │                │              │  ├ POST /auth/*    │
          │                │              │  ├ PUT  /users/*/  │
          │                │              │  │      avatar     │
          │                │              │  └ GET  /ws ───────┤
          │                │              │   WS upgrade + NMP │
          │                │              └─────────┬──────────┘
          │                │                        │ NMP (interno)
     ┌────▼────────────────▼────────────────────────▼──────┐
     │                 Arc<AppState>                        │
     │                                                      │
     │  ┌─────────────┐   ┌──────────────┐                 │
     │  │ NMP Handler │   │SessionManager│                 │
     │  └─────────────┘   └──────────────┘                 │
     │  ┌─────────────┐   ┌──────────────┐                 │
     │  │ Room/DM Bus │   │PresenceEngine│                 │
     │  └─────────────┘   └──────────────┘                 │
     │  ┌──────────────────────────────────────────────┐   │
     │  │                Message Store                 │   │
     │  └──────────────────────────┬───────────────────┘   │
     └─────────────────────────────┼────────────────────────┘
                                   │
                       ┌───────────▼──────────┐
                       │    PostgreSQL DB      │
                       └──────────────────────┘
```

O cliente web conecta via WebSocket no mesmo listener axum (`:8080/ws`). O handler de WebSocket dentro do axum faz a tradução WS↔NMP internamente, sem processo separado, e repassa os frames para o mesmo `NMP Handler` que atende o TCP.

---

## Protocolo: NMP (NeoMSN Protocol)

Protocolo binário sobre TCP. Cada frame tem o formato:

```
┌──────────┬──────────┬───────────────────┬──────────────┐
│  Magic   │ Opcode   │  Payload Length   │   Payload    │
│  2 bytes │  1 byte  │     4 bytes       │   N bytes    │
└──────────┴──────────┴───────────────────┴──────────────┘
Magic: 0x4E4D ("NM")
```

### O que NÃO entra no protocolo (HTTP REST)

Operações que ocorrem antes ou fora de uma sessão autenticada ficam em endpoints HTTP separados. O JWT obtido via HTTP é então passado no opcode `AUTH`.

| Endpoint           | Método | Descrição                                              |
|--------------------|--------|--------------------------------------------------------|
| `/auth/signup`     | POST   | Criar conta (username, password, display_name)         |
| `/auth/login`      | POST   | Autenticar, retorna JWT + device_id                    |
| `/auth/logout`     | POST   | Invalidar token no servidor                            |
| `/users/:id/avatar`| PUT    | Upload de avatar (multipart — inadequado para binário) |

### Opcodes

Os opcodes cobrem tudo que acontece dentro de uma sessão NMP autenticada. Agrupados por faixa:

| Faixa  | Grupo           |
|--------|-----------------|
| 0x01–0x0F | Sessão / Handshake |
| 0x10–0x1F | Mensagens (streaming) |
| 0x20–0x2F | Salas (Rooms) |
| 0x30–0x3F | Chats diretos (DM) |
| 0x40–0x4F | Presença |
| 0x50–0x5F | Sincronização |
| 0x60–0x6F | Perfil do usuário |
| 0x70–0x7F | Contatos |
| 0xF0–0xFF | Sistema (ping, erro) |

#### Sessão / Handshake

| Código | Nome              | Direção   | Descrição                                      |
|--------|-------------------|-----------|------------------------------------------------|
| 0x01   | HELLO             | C→S       | Handshake com versão do protocolo e device_id  |
| 0x02   | AUTH              | C→S       | Enviar JWT obtido via HTTP                     |
| 0x03   | AUTH_OK           | S→C       | Sessão estabelecida, retorna dados do usuário  |
| 0x04   | AUTH_FAIL         | S→C       | Token inválido ou expirado                     |

#### Mensagens (streaming)

| Código | Nome              | Direção       | Descrição                                      |
|--------|-------------------|---------------|------------------------------------------------|
| 0x10   | MSG_CHUNK         | C→S / S→C     | Edição do texto: `truncate_to` (bytes) + `delta` |
| 0x11   | MSG_COMPLETE      | C→S / S→C     | Mensagem finalizada (botão Concluir)           |
| 0x12   | MSG_DELETE        | C→S / S→C     | Mensagem cancelada antes de ser concluída      |

`MSG_CHUNK` carrega qualquer edição, não só acréscimos: o receptor trunca o texto
acumulado para `truncate_to` bytes (o prefixo comum com o estado anterior) e anexa
`delta`. Digitação normal é `truncate_to == tamanho atual`; backspace é um
`truncate_to` menor com `delta` vazio. Apagar todo o texto envia `MSG_DELETE`.

#### Salas (Rooms)

| Código | Nome              | Direção   | Descrição                                      |
|--------|-------------------|-----------|------------------------------------------------|
| 0x20   | ROOM_CREATE       | C→S       | Criar nova sala (nome, descrição)              |
| 0x21   | ROOM_CREATE_OK    | S→C       | Sala criada, retorna room_id                   |
| 0x22   | ROOM_UPDATE       | C→S       | Renomear sala ou alterar descrição             |
| 0x23   | ROOM_DELETE       | C→S       | Deletar sala (somente criador/admin)           |
| 0x24   | ROOM_JOIN         | C→S       | Entrar em uma sala existente                   |
| 0x25   | ROOM_LEAVE        | C→S       | Sair de uma sala                               |
| 0x26   | ROOM_LIST         | C→S       | Solicitar lista de salas disponíveis           |
| 0x27   | ROOM_LIST_RESP    | S→C       | Resposta com lista de salas                    |
| 0x28   | ROOM_MEMBERS      | S→C       | Membros atuais de uma sala                     |
| 0x29   | ROOM_EVENT        | S→C       | Evento de sala (membro entrou/saiu/foi banido) |
| 0x2A   | CHAT_INVITE       | C→S       | Convidar usuário para a conversa atual (DM ou sala) |
| 0x2B   | CHAT_JOINED       | S→C       | Entrou numa conversa em grupo; lista de membros |

**Conversas em grupo (estilo MSN):** não existe criação explícita de grupo. A partir
de um DM, qualquer participante envia `CHAT_INVITE` com o `user_id` de um contato
**online**. O servidor cria uma sala efêmera com os três participantes e envia
`CHAT_JOINED` a todos (com `origin_context_id` = o DM de origem, para que janelas
abertas convertam o contexto em vez de abrir outra janela). Convites subsequentes
adicionam membros à sala (`CHAT_JOINED` para o convidado, `ROOM_EVENT` para os
demais). Fechar a janela envia `ROOM_LEAVE`; desconexão também conta como saída.
A sala morre naturalmente quando todos saem — como no MSN.

#### Chats Diretos (DM)

| Código | Nome              | Direção   | Descrição                                      |
|--------|-------------------|-----------|------------------------------------------------|
| 0x30   | DM_OPEN           | C→S       | Abrir conversa direta com outro usuário        |
| 0x31   | DM_OPEN_RESP      | S→C       | Confirma abertura do DM, retorna context_id    |

#### Presença

| Código | Nome              | Direção   | Descrição                                         |
|--------|-------------------|-----------|---------------------------------------------------|
| 0x40   | PRESENCE_SET      | C→S       | Atualizar próprio status (online/ausente/ocupado) |
| 0x41   | PRESENCE_UPDATE   | S→C       | Status de um contato mudou                        |

#### Sincronização

| Código | Nome              | Direção   | Descrição                                         |
|--------|-------------------|-----------|---------------------------------------------------|
| 0x50   | SYNC_REQUEST      | C→S       | Solicitar histórico de um contexto (últimas N mensagens `complete`) |
| 0x51   | SYNC_RESPONSE     | S→C       | Lote de mensagens persistidas, em ordem cronológica |

O cliente envia `SYNC_REQUEST { context_type, context_id, limit }` ao abrir uma
janela de conversa; o servidor responde com as mensagens `complete` mais recentes
lidas do banco (id, autor, nome do autor, conteúdo).

#### Perfil do Usuário

Operações que alteram dados do próprio usuário autenticado. Leitura de perfil alheio
também passa aqui para manter tudo no canal persistente.

| Código | Nome              | Direção   | Descrição                                         |
|--------|-------------------|-----------|---------------------------------------------------|
| 0x60   | PROFILE_GET       | C→S       | Buscar perfil de um usuário (por user_id)         |
| 0x61   | PROFILE_RESP      | S→C       | Dados do perfil (display_name, bio, avatar_url)   |
| 0x62   | PROFILE_UPDATE    | C→S       | Atualizar display_name ou mensagem pessoal        |
| 0x63   | PROFILE_UPDATE_OK | S→C       | Confirmação de atualização de perfil              |

#### Contatos

| Código | Nome              | Direção   | Descrição                                         |
|--------|-------------------|-----------|---------------------------------------------------|
| 0x70   | CONTACT_LIST      | C→S       | Solicitar lista de contatos do usuário            |
| 0x71   | CONTACT_LIST_RESP | S→C       | Lista de contatos com status de presença atual    |
| 0x72   | CONTACT_ADD       | C→S       | Adicionar contato (por username)                  |
| 0x73   | CONTACT_ADD_OK    | S→C       | Contato adicionado; começa a receber presença     |
| 0x74   | CONTACT_REMOVE    | C→S       | Remover contato da lista                          |
| 0x75   | CONTACT_BLOCK     | C→S       | Bloquear usuário (não recebe msgs/presença)       |
| 0x76   | CONTACT_UNBLOCK   | C→S       | Desbloquear usuário                               |
| 0x77   | CONTACT_REQUEST   | S→C       | Outro usuário quer te adicionar como contato      |

#### Sistema

| Código | Nome              | Direção       | Descrição                                      |
|--------|-------------------|---------------|------------------------------------------------|
| 0xF0   | ERROR             | S→C           | Erro genérico com código e mensagem            |
| 0xFF   | PING / PONG       | Bidirecional  | Keepalive                                      |

### Fluxo de mensagem em streaming

```
Usuário digita "Olá"       → MSG_CHUNK  { msg_id: UUID, room/dm, truncate_to: 0, delta: "O" }
                           → MSG_CHUNK  { msg_id: UUID, room/dm, truncate_to: 1, delta: "l" }
                           → MSG_CHUNK  { msg_id: UUID, room/dm, truncate_to: 2, delta: "á" }
Backspace ("Ol")           → MSG_CHUNK  { msg_id: UUID, room/dm, truncate_to: 2, delta: "" }
Usuário clica em Concluir  → MSG_COMPLETE { msg_id: UUID }
Próxima digitação          → MSG_CHUNK  { msg_id: NOVO_UUID, ... }
```

O servidor retransmite cada chunk em tempo real para todos os participantes da conversa.

---

## Stack Tecnológica

### Servidor (`neomsn-server`)
- **Rust** — tokio (async runtime)
- **tokio::net::TcpListener** — listener NMP na porta 7777
- **axum** — listener HTTP/WebSocket na porta 8080 (REST + upgrade WS para clientes web)
- O handler WebSocket do axum traduz WS↔NMP internamente e compartilha o mesmo `NMP Handler` do TCP
- **SQLx** + **PostgreSQL** — armazenamento de mensagens, usuários, salas
- **argon2** — hash de senhas
- **jsonwebtoken** — autenticação stateless via JWT

### Clientes (`neomsn-desktop`, `neomsn-mobile`, `neomsn-web`)
- **Iced** em todos os três — framework de UI reativo com suporte a desktop, mobile (Android/iOS) e WASM
- Os widgets compartilhados vivem em `neomsn-shared` e são reutilizados por todos os clientes
- Cada crate de cliente monta o layout e gerencia APIs de plataforma; não define widgets próprios
- `neomsn-web` compila para WASM e conecta via WebSocket (`:8080/ws`)

---

## Estrutura do Workspace

```
neomsn/
├── CLAUDE.md
├── Cargo.toml                      # workspace root
├── crates/
│   ├── neomsn-shared/              # tudo que é comum aos três clientes
│   │   ├── src/
│   │   │   ├── proto/
│   │   │   │   ├── frame.rs        # parsing/serialização de frames NMP
│   │   │   │   ├── opcodes.rs      # enum de opcodes
│   │   │   │   └── types.rs        # structs do protocolo (MsgChunk, etc.)
│   │   │   ├── domain/
│   │   │   │   ├── user.rs         # User, PresenceStatus
│   │   │   │   ├── message.rs      # Message, MessageChunk
│   │   │   │   └── room.rs         # Room, RoomMember
│   │   │   └── widgets/            # widgets Iced compartilhados
│   │   │       ├── contact_item.rs # item de contato com avatar e status
│   │   │       ├── chat_bubble.rs  # balão de mensagem (streaming + completo)
│   │   │       ├── status_dot.rs   # indicador colorido de presença
│   │   │       ├── emoji_picker.rs # painel de seleção de emojis
│   │   │       └── theme.rs        # paleta e estilos MSN para Iced
│   │   └── Cargo.toml              # depende de iced; compila para native e WASM
│   │
│   ├── neomsn-server/              # servidor (porta 7777 NMP + porta 8080 HTTP/WS)
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── nmp/
│   │   │   │   ├── listener.rs     # TcpListener :7777
│   │   │   │   └── session.rs      # estado por conexão autenticada
│   │   │   ├── http/
│   │   │   │   ├── router.rs       # axum routes (:8080)
│   │   │   │   ├── auth.rs         # POST /auth/signup, /auth/login
│   │   │   │   ├── avatar.rs       # PUT /users/:id/avatar
│   │   │   │   └── ws.rs           # GET /ws — upgrade + tradução WS↔NMP
│   │   │   ├── state/
│   │   │   │   ├── presence.rs     # engine de presença
│   │   │   │   └── rooms.rs        # bus de salas e roteamento de mensagens
│   │   │   └── db/                 # queries SQLx
│   │   └── Cargo.toml
│   │
│   ├── neomsn-desktop/             # cliente Linux/Windows/macOS
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── app.rs              # Application Iced — update loop
│   │   │   ├── net/
│   │   │   │   └── tcp.rs          # conexão NMP via TCP nativo
│   │   │   └── screens/
│   │   │       ├── login.rs
│   │   │       ├── contact_list.rs # janela principal estilo MSN
│   │   │       └── chat.rs         # janela de chat (pode ter múltiplas)
│   │   └── Cargo.toml
│   │
│   ├── neomsn-mobile/              # cliente Android/iOS
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── app.rs
│   │   │   ├── net/
│   │   │   │   └── tcp.rs          # conexão NMP via TCP (mesma lógica, target diferente)
│   │   │   └── screens/
│   │   │       ├── login.rs
│   │   │       ├── contact_list.rs # layout adaptado para toque/tela pequena
│   │   │       └── chat.rs         # tela de chat (navegação em stack, não janelas)
│   │   └── Cargo.toml
│   │
│   └── neomsn-web/                 # cliente WASM
│       ├── src/
│       │   ├── lib.rs
│       │   ├── app.rs
│       │   ├── net/
│       │   │   └── ws.rs           # NMP encapsulado em WebSocket
│       │   └── screens/
│       │       ├── login.rs
│       │       ├── contact_list.rs
│       │       └── chat.rs
│       └── Cargo.toml
│
├── migrations/                     # migrações SQL (SQLx)
└── assets/                         # ícones, sons, emoticons estilo MSN
```

### Dependências entre crates

```
neomsn-shared   ←── neomsn-desktop
                ←── neomsn-mobile
                ←── neomsn-web
                ←── neomsn-server   (somente proto/ e domain/; não importa widgets)
```

`neomsn-shared` depende de **iced** e deve compilar tanto para targets nativos quanto para `wasm32-unknown-unknown`. Dependências com syscalls (tokio, SQLx) ficam fora dele.

---

## Modelo de Dados

Entidades do domínio e seus relacionamentos. Independente de banco de dados.

---

### User

Representa uma conta. O `username` é o identificador público usado para adicionar contatos e fazer login.

```
User
├── id: Uuid
├── username: String          -- único; usado em CONTACT_ADD e login
├── display_name: String      -- nome exibido na UI (pode mudar)
├── personal_message: String  -- mensagem pessoal estilo MSN (pode ser vazia)
├── avatar: AvatarRef         -- referência ao avatar (url ou id de blob)
├── password_hash: String     -- armazenado, nunca trafegado
├── created_at: Timestamp
└── deleted_at: Option<Timestamp>  -- soft delete
```

---

### Device

Cada dispositivo autenticado do usuário tem seu próprio registro. É a unidade de sincronização: cada `Device` tem um cursor por contexto indicando até onde recebeu mensagens.

```
Device
├── id: Uuid
├── user_id: Uuid → User
├── name: String              -- ex: "Meu PC", "Galaxy S24"
├── platform: Platform        -- Desktop | Mobile | Web
└── last_seen_at: Timestamp
```

---

### Contact

Relacionamento direcional entre usuários — assim como no MSN, adicionar alguém não é automático: a outra parte recebe uma solicitação. Cada lado tem sua própria entrada com seu próprio estado.

```
Contact
├── owner_id: Uuid → User     -- quem tem este contato na lista
├── contact_id: Uuid → User   -- o contato em si
├── state: ContactState       -- Pending | Accepted | Blocked
└── since: Timestamp          -- quando entrou neste estado
```

```
ContactState
├── Pending   -- solicitação enviada, aguardando aceitação
├── Accepted  -- contato confirmado por ambos os lados
└── Blocked   -- owner bloqueou contact; contact não sabe
```

Invariante: presença e mensagens só fluem entre pares onde ambos os lados têm `state = Accepted`.

---

### Presence

Status de presença de um usuário. É majoritariamente estado em memória no servidor, mas o último status conhecido é persistido para exibição quando o usuário está offline.

```
Presence
├── user_id: Uuid → User
├── status: PresenceStatus    -- Online | Away | Busy | Invisible | Offline
└── updated_at: Timestamp
```

```
PresenceStatus
├── Online
├── Away
├── Busy
├── Invisible   -- aparece como Offline para contatos, mas recebe mensagens
└── Offline     -- sem sessão ativa
```

---

### Room

Sala de grupo. Qualquer membro pode entrar com o nome da sala; o criador é o dono inicial.

```
Room
├── id: Uuid
├── name: String
├── description: String
├── created_by: Uuid → User
├── created_at: Timestamp
└── deleted_at: Option<Timestamp>
```

---

### RoomMember

Participação de um usuário em uma sala. `left_at` nulo significa membro ativo.

```
RoomMember
├── room_id: Uuid → Room
├── user_id: Uuid → User
├── role: RoomRole            -- Member | Admin | Owner
├── joined_at: Timestamp
└── left_at: Option<Timestamp>
```

```
RoomRole
├── Member  -- pode enviar e ler mensagens
├── Admin   -- pode renomear a sala e remover membros
└── Owner   -- pode deletar a sala; único por sala
```

---

### DirectConversation

Conversa direta entre dois usuários. Garante que existe exatamente um contexto de DM para cada par, independente de quem iniciou. O `id` gerado é determinístico a partir dos dois `user_id` (menor UUID + maior UUID), mas armazenado para referência estável.

```
DirectConversation
├── id: Uuid                  -- context_id usado em Message
├── user_a: Uuid → User       -- sempre o menor UUID dos dois
├── user_b: Uuid → User       -- sempre o maior UUID dos dois
└── created_at: Timestamp
```

---

### Message

Unidade central de conteúdo. O `id` é gerado pelo **cliente** no momento em que o usuário começa a digitar — antes de qualquer chunk ser enviado. O `content` é acumulado conforme os chunks chegam e finalizado em `Complete`.

```
Message
├── id: Uuid                        -- gerado pelo cliente ao abrir o campo de texto
├── context: MessageContext         -- onde a mensagem foi enviada
├── author_id: Uuid → User
├── content: String                 -- texto acumulado; vazio até o primeiro chunk
├── status: MessageStatus
├── started_at: Timestamp           -- quando o cliente criou o msg_id
└── completed_at: Option<Timestamp> -- quando MSG_COMPLETE foi recebido
```

```
MessageContext
├── Room { room_id: Uuid }
└── Direct { conversation_id: Uuid }
```

```
MessageStatus
├── Streaming   -- chunks sendo recebidos; visível em tempo real
├── Complete    -- finalizada pelo usuário (MSG_COMPLETE)
└── Deleted     -- cancelada antes de completar (MSG_DELETE)
```

---

### MessageChunk

Fragmento de texto de uma mensagem em streaming. Permite replay do streaming para dispositivos que reconectam durante uma mensagem ativa, e sincronização de mensagens incompletas no histórico.

Chunks são **descartáveis** após a mensagem atingir `Complete` — o `content` acumulado em `Message` é suficiente para o histórico. Podem ser purgados em background.

```
MessageChunk
├── id: u64                   -- sequencial global por contexto; usado como cursor de sync
├── message_id: Uuid → Message
├── delta: String             -- fragmento de texto (um ou mais caracteres)
├── seq: u32                  -- posição do chunk dentro da mensagem (começa em 0)
└── created_at: Timestamp
```

---

### SyncCursor

Marca até onde cada dispositivo recebeu eventos em cada contexto. Usado no handshake de reconexão (opcode `SYNC_REQUEST`) para entregar exatamente o que o dispositivo perdeu.

```
SyncCursor
├── device_id: Uuid → Device
├── context_id: Uuid          -- room_id ou DirectConversation.id
└── last_chunk_id: u64        -- último MessageChunk.id recebido por este device
```

---

### Relacionamentos resumidos

```
User ──< Device
User ──< Contact >── User          (direcional, dois registros por par aceito)
User ──< RoomMember >── Room
User ──< DirectConversation >── User
User ──< Message (author)

Room ──< Message
DirectConversation ──< Message

Message ──< MessageChunk

Device ──< SyncCursor >── (Room | DirectConversation)
```

---

### O que NÃO é persistido

| Dado | Motivo |
|---|---|
| Indicador "está digitando" | Ephemeral — estado do MSG_CHUNK ativo em memória no servidor |
| Sessões NMP ativas | Estado de processo; reconstruído no reconnect via SyncCursor |
| Subscriptions de presença | Derivado da lista de contatos aceitos em memória |
| Chunks de mensagens Complete | Purgáveis após acumular em `Message.content` |

---

## UI: Inspiração MSN

### Janela principal (Lista de Contatos)
- Barra de título com nome do usuário e foto
- Seletor de status: Online / Ausente / Ocupado / Invisível (com ícones coloridos)
- Lista de contatos agrupada (Online / Ausente / Offline)
- Ícone de status colorido ao lado de cada contato
- Busca de contatos

### Janela de Chat
- Histórico de mensagens com balões
- Mensagens de outros usuários aparecem caractere por caractere em tempo real
- Campo de texto na parte inferior (sem botão Enviar)
- Botão **Concluir** (ou Enter configurável) finaliza a mensagem
- Indicação visual de "está digitando..." enquanto o outro usuário digita
- Emojis: botão na barra do campo de texto abre um painel para injetar emoji na mensagem

### Tema
- Paleta baseada no MSN 7/2009: gradientes azuis, branco, cinza claro
- Versão dark opcional (modernização)
- Fonte: Tahoma ou equivalente (Segoe UI como fallback)

---

## Comportamento do Streaming de Texto

1. Ao abrir o campo de texto, o cliente gera um `msg_id` (UUID v4).
2. Cada edição do campo (digitação, backspace, edição no meio) envia um `MSG_CHUNK` com `truncate_to` (prefixo comum em bytes) + `delta` ao servidor.
3. O servidor retransmite o chunk para todos os participantes da conversa.
4. Todos os participantes veem o texto aparecer — e ser apagado — em tempo real.
5. Ao clicar **Concluir** (ou pressionar a tecla configurada), envia `MSG_COMPLETE`.
6. O servidor persiste a mensagem completa no banco e descarta os chunks intermediários (ou os arquiva conforme configuração).
7. O cliente gera um novo `msg_id` para a próxima mensagem.
8. Se o usuário apagar todo o texto, envia `MSG_DELETE` para limpar o estado nos outros clientes; um novo `msg_id` é gerado para a próxima digitação.

---

## Sincronização entre Dispositivos

- Cada dispositivo tem um `device_id` e um cursor de sincronização por contexto (sala/DM).
- Ao reconectar, o cliente envia `SYNC_REQUEST` com o último cursor conhecido.
- O servidor responde com `SYNC_RESPONSE` contendo as mensagens/chunks perdidos em ordem.
- Mensagens em status `streaming` de outros dispositivos do mesmo usuário são descartadas na sincronização (somente `complete` é sincronizado entre dispositivos do mesmo usuário).

---

## Convenções de Desenvolvimento

- Código em **inglês** (nomes de variáveis, funções, comentários internos)
- Comentários somente quando o "porquê" não é óbvio
- Sem tratamento de erros para casos impossíveis — usar `unwrap`/`expect` com mensagem clara onde o erro indicaria bug, não condição de runtime
- Sem abstrações prematuras — implementar o caso concreto primeiro
- Testes de integração para o protocolo NMP usam conexões TCP reais, sem mocks de socket
- Migrações SQL versionadas com SQLx e commitadas no repositório

---

## Comandos Úteis

```bash
# Rodar o servidor em modo desenvolvimento
cargo run -p neomsn-server

# Rodar o cliente desktop
cargo run -p neomsn-client

# Build WASM do cliente web
wasm-pack build crates/neomsn-web --target web

# Rodar migrações
sqlx migrate run

# Checar tudo
cargo check --workspace
cargo test --workspace
cargo clippy --workspace
```
