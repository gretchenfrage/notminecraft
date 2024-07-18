
# Game loops

The client and the server both operate as a single-threaded event loop which
process events including regularly scheduled ticks, messages received from the
network, input from users, and previously started asynchronous operations
completing.

## Server game loop

The server attempts to commit to a schedule of performing a "tick" every 50 ms.
A tick is just an opportunity to run whatever game logic needs to be run
repeatedly on the server every 50 ms.

50 ms is not the targeted time between the end of one tick and the beginning of
the next, but rather, between the beginning of one tick and the beginning of
the next. As such, unless the server is overwhelmed, ticks run at an entirely
pre-determined and regular schedule of X, X+50ms, X+100ms, etc.

    tick 1  tick 2  tick 3
    |--|    |---|   |--|
    <------><------><------>
    50 ms   50 ms   50 ms

The exception to this regular schedule is if the computations for a tick take
longer than 50 ms. If this occurs, the server simply decides to "skip" a
certain number of ticks so that it's no longer behind schedule.

    tick 1  tick 2  skip    skip    tick 3
    |--|    |------------------|    |--|
    <------><----------------------><------>
    50 ms   150 ms                  50 ms

As such, even in the event that the server cannot keep up with its targeted
schedule of 20 ticks per second, each tick will still begin at an integer
multiple of 50 ms after the first tick.

Server game logic should always treat a tick as 50 ms, even if ticks were
skipped.

In between ticks, the server tries to process other events as they come in.
As such, the server loop may do something like: do a tick, process some events
that came in, go to sleep, be woken up by some more events coming in, process
them, go to sleep, be woken up by it being time to do the next tick, and go on
as such.

The server does not currently have a system for cutting off event processing
to avoid delaying a tick. Once the server begins dequeueing and processing
events, it continues to do so until doing so would block.

## Client game loop

The client has a target frequency at which it tries to do "frames", which
consist of an "update," which is just an opportunity to run whatever logic
needs to be run right before rendering, followed by rendering to the window.
The client automatically sets this frequency to the detected monitor refresh
rate. Client update logic may be used for things like smooth animations,
interpolation, and client-side prediction. Unlike a server tick, client updates
have a variable time step--one parameter to the update function is the float
number of seconds since the beginning of the last call to the update function.

Much like the server game loop, the client game loop can be woken up in between
frames to process other incoming events. However, the client game loop has more
complex strategies to ensure that even if rendering, processing other events,
or both are overwhelming the system in terms of how much time they're taking,
neither will entirely starve the other:

- When doing a frame, the client tracks when the frame began and when the frame
  ended, and the duration between them, the "frame duration".
- After each frame, we consider "next frame target" instant to be the last
  frame's start instant plus the inverse of the target frequency.
- After each frame, we allow the client to dequeue and process queued events
  until whichever instant is _later_ between:

  1. The next frame target instant.
  2. Half the last frame duration after the last frame ended.

  If that instant is reached before the client finishes processing queued
  events, then it stop processing events and runs the next frame, then repeats.
  If the client exhausts its queued events before that instant is reached, then
  it repeatedly sleeps until either the next frame target instant is reached,
  at which point it runs the next frame, or until some event arrives before the
  next frame target instant, at which point it processes that event.

This gives it the following ways of responding to system stress:

- Nothing stressed: Rendering happens at target frequency, events processed
  as they come in, and it spends the rest of its time sleeping.
- Only rendering stressed: Rendering happens at nearly the maximal frequency
  rendering could happen (less than the target frequency), and between each
  frame some events may be quickly processed.
- Only event processing stressed: Rendering happens at the target frequency,
  and all the time between frames is spent processing events. A backlog of
  queued events may form.
- Both rendering and event processing stressed: Rendering happens at two thirds
  of the maximal frequency rendering could happen (itself less than the target
  frequency). The remaining third of time between frames is spent processing
  events. A backlog of queued events may form.


# ~~Time-like things~~ Warning: This Is All Wrong

The multiplayer protocol deals with 3 different time-like things:

- Game tick numbers
- Real time
- Client message acks

## Game tick numbers and real time 

Ticks that the server does are given sequential `u64` tick numbers. Skipped
ticks do not correspond to tick numbers--the tick number increases only once
for each time a tick actually happens.

Real time refers to the passage of real wall-clock time. The network protocol
performs server/client clock synchronization as part of their handshake so that
they then have a common temporal reference point that they can transmit
timestamps relative to. This allows conversion on both the client and server
between `Instant`s and an `i64` number microseconds after their mutually
synchronized reference point.

Within the server loop, game logic generally should not directly call
`Instant::now`. This is based on the principle that the game logic should, to
a certain extent, act as if that computations required for that game logic
complete instantenously, and thus that the entire computation occurred at the
instant it started. As such, any game logic running within a tick should only
reference the exact instant the tick was scheduled to begin (after accounting
for skipped ticks), and any game login running within the processing of some
other event the server received should only reference a single `Instant::now`
sampled at the beginning of when the server started processing that event.

When the client and server initialize, the server tells the client its last
tick number and the instant that tick was scheduled to start. Every time the
server the server does a tick, before sending clients any other messages, it
sends all clients a message indicating that it is beginning a tick with the
given tick number and with a certain number of skipped ticks beforehand. The
tick number part of the message is just for defensiveness, and the client
should treat it as a protocol error if they don't increase by one every time.

The client maintains a "down stream time" instant, a monotonically increasing
instant which's current value upon the client receiving a message should
correspond to the instant the server considered to be the present when it sent
that message. When the client receives a message from the server indicating
the start of a new tick as described above, it updates its down stream time to
match the instant was scheduled to begin on the server (after accounting for
skipped ticks).

When the server begins processing some other event, it may send a different
type of message to the client specifying the current server time to be an
arbitrary real time instant, the one the server sampled at the beginning of
processing the event. The server may transmit these lazily, deferring doing so
until right before the server sends some other message to the client and
realizes that the value the client's down stream time will hold upon receiving
that message is outdated. The client should treat down stream time instants not
monotonically increasing as a protocol error.

## Client message acks

Messages sent from the client to the server are each designated an "up message
index" within that connection, starting at _1_ for the first and counting
upwards. After the server messages processing a message from a client, it
sends that client a message acknowledging the up message index of the message
it just processed, whether or not the server sent any other messages to that
client in the course of processing the message.


TODO:

- note that client does the same sort of thing with instants
- note on how client can't really queue window and device events
