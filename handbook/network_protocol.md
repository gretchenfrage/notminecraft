# Network Protocol

### Transport

The network protocol is binschema messages sent over a websocket connection.
Sending the schema upon connection initialization is currently a TODO item. See
the `minecraft::message.rs` module for schema definition.

### Logging in and joining game

When a connection is made to the server, the server considers it to be in the
"uninit" state. In the uninit state the connection can only send the server a
LogIn message, which, if accepted, transitions the connection to the "client"
state, where it spends most of its lifespan. However, the client connection is
not yet "in game".

As soon as the connection transitions to the client state, the server sends it
an AcceptLogin message and then may begin sending the client messages to load
and keep updated the world and its contents on that client. After the server
has sent the client the initial region of world surrounding where it will
appear, the server sends a ShouldJoinGame message, upon which the client SHOULD
join the game.

A client connection which is not in game can only send a JoinGame message, and
a connection can only send a JoinGame message once. When the server processes
the JoinGame message considers the player to be in the game and places the
player's character in the world and begins simulating it.

Before the client joins the game, the client should display to the user some
sort of loading screen. However, this is currently a TODO item.

### Closing

The server can close a connection elegantly by sending a Close message. This
contains a string message which the client should display to the user, however,
this is currently a TODO item. This transitions the connection to the "closed"
state, in which the server will not process any messages from the client. In
some cases the server may handle an invalid message received from a connection
by closing the connection, and in other cases it may simply ignore the message.
A client can leave the game by simply terminating the transport.

### Adding and removing chunks

There is a set of chunks loaded on the server, and a corresponding space of
server-side cis (chunk-indices). For more information on chunk coordinates
versus chunk indices and other related concepts, please see the top-level
API docs for the `chunk_data` package.

Each client has its own set of chunks that are loaded on that client, which
is a subset of the server-side chunk set (modulo asynchrony delays). As such,
each client has its own client-side ci space. A client does not track the
server-side cis for chunks--a server always translates cis into a client's ci
space before sending a message to that client.

A server sends a client an AddChunk message to add a chunk to that client.
Although AddChunk contains both a cc and ci, the ci is only for fail-fast
reasons--the adding and removing of chunks to that client must follow a slab
pattern of ci assignment, and the client treats the server doing otherwise
as a terminal protocol violation error.

While a chunk exists for the client, the server sends that client edits to the
chunk as they happen server-side to keep it up to date.

When a client is sending the server a message about a particular chunk, it
usually references the chunk by cc (chunk coordinate) rather than ci, for race
condition reasons.

### Chunk loading rate limiting

Currently, the network transport constitutes a single pair of FIFO message
queues between the client and the server. Furthermore, both sides have an
infinitely  growable send queue rather than dealing with backpressure or
message dropping. To deal with some potential problems arising from this, a
form of rate limiting is implemented specifically for the server sending the
client new chunks.

The server tracks for each client connection a "load chunk budget." This can
be thought of more or less as the maximum number of chunks that are allowed to
be "in transit" to be loaded into the client at a given time. It is initialized
to some initial value, such as 20. When the server enqueues a new chunk to be
transmitted to the client it decrements the budget. The server avoids making
the budget negative, and thus if the budget reaches 0 the server will postpone
sending additional new chunks until the budget is increased.

TODO the initial value should be made configurable, as it is appropriate to
scale it to match server-to-client network bandwidth. Alternative approaches
include, attempting to dynamically determine network bandwidth... or just,
using QUIC.

When the client receives a chunk and inserts it into the world, it sends the
server an AcceptMoreChunks message. When the server receives this is increases
the budget accordingly. However, the client MUST NOT try to AcceptMoreChunks
a greater amount than it has receives AddChunks. The server attempts to detect
this and terminates the client connection if so.

This achieves 2 things:

- Keeps server-to-client message receipt delay low by preventing messages from
  being buried under a huge queue of chunks to be sent.
- Protects against a slow read denial of service wherein the client could
  trigger the server to keep sending queue it chunks to send without reading
  the stream, causing the server to run out of memory.

### Edits and client-side prediction

As the server edits state which is also replicated on the clients, the server
sends the clients which have that state loaded ApplyEdit messages containing
representations of the edits which the clients can apply to their local copies
to keep it synchronized.

Within a connection, all messages sent upwards from the client to the server
have an up msg index, wherein the first message sent up has the index 1. This
starts counting from the very first message sent upwards in the connection,
not just when the connection enters the client state. When the server processes
messages from the client, it sends an Ack message, or at or some other message
containing an ack field, indicating the maximum messages from the client it's
processed.

That is enough for the client to implement client-side prediction. TODO explain
that more.

### Menus


