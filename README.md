# Motors

This repository contains 4 crates related to board games:

- `gears`: **Board representation** and general utility
- `motors`: **Engines**
- `monitors`: a ***UGI client**, will eventually include a GUI (text-based for now)
- `pliers`: **Tuner** for HCE eval weights

Currently, the most interesting part is probably the superhuman UCI [Chess960](<https://en.wikipedia.org/wiki/Fischer_random_chess>), DFRC
and **Chess Engine** `CAPS`.

## CAPS

A chess engine estimated at >= 3k elo when used with a hand-crafted evaluation function.

Current features:

- Alpha-beta Pruning Negamax
- Quiescent Search
- Move Ordering:
    - TT Move
    - Killer Move
    - Various History Heuristics
    - MVV and Capture History
    - SEE to partition captures into good and bad captures
- Transposition Table
- Iterative Deepening
- Internal Iterative Reductions
- Aspiration Windows
- Principal Variation Search
- Check Extensions
- Null Move Pruning with Verification Search
- Reverse Futility Pruning
- Futility Pruning
- Late Move Pruning
- SEE pruning in quiescent search
- Adjust pruning and reduction margins based on eval difference to last move
- Time Management with a soft and hard bound, as well as support for fixed time, depth, nodes, mate and `infinite`, or any combination of
  these
    - Almost full UCI compliance, including `searchmoves`, `ponder`, `multipv`, etc. Notable missing features are endgame tablebases and a
      built-in opening book.
- Eval function can be changed at runtime (see below)
- ## Other Engines

In addition to the **C**hess-playing **A**lpha-beta **P**runing **S**earch (**CAPS**),
there is also the **G**eneral **A**lpha-beta **P**runing **S**earch (**GAPS**, a game-agnostic engine, currently still very basic),
and **random** (a random mover).
Except for **random**, those engines can be combined with any evaluation function supporting the current game.

They are currently being worked on and will be stronger in the future.
There will also be additional engines, like **M**inimalistic **A**lpha-beta **P**runing **S**earch (**MAPS**),
a simple alpha-beta pruning search without any further techniques, or an MCTS implementation.

## Evaluation Functions

### MateOnCE

**Mate**rial **On**ly **C**hess **E**val, a material-only evaluation function for chess,
using the classical piece values 1, 3, 3, 5, 9.

### PiSTOn

**Pi**ece **S**quare **T**able **On**ly eval, a chess eval using only piece square tables,
similar to the well-known PeSTO engine.

### LiTE

**Li**near **T**uned **Eval**, a chess eval using a linear combination of weights which have
been tuned using the *pliers* tuner.
It can be interpreted as a single layers perceptron, a neural net consisting of a single neuron.

### BasE

**Bas**ic m,n,k **E**val, a simple hand-crafted eval for m,n,k games.

### random

Returns random values. Still stronger than the *random* engine when used as eval function
for an actual engine like *caps* or *gaps*.

## Games

Currently, there are 3 games implemented:

- **Chess**, including Chess960 (a.k.a. Fischer Random Chess) and Double Fischer Random Chess (DFRC)
- **Ataxx**, a challenging board game where the goal is to convert your opponent's pieces
- **m, n, k** games, a generalization of Tic-Tac-Toe that can actually be difficult. The current implementation is somwewhat limit and does
  not support boards larger than 128 squares, nor does it (yet) support rules specific to variants such as Connect 4 or Gomoku.

## Usage

### Building

To build the engines, it is enough to type `make` or use `cargo`.
`cargo` can also be used to build other parts, such as the match manager `monitors` or the tuner `pliers`.
Individual games and engines can be included or excluded from the build through cargo features, but the default is to build everything.
Alternatively, I'm planning to do a github release soon, which will contain at least the engines binary.

### Running

#### Manual User Input

All engines use the UCI or the very similar and mostly compatible UGI protocol for communicating with the GUI.
But this interface has also been designed to be easy to use for a human.
Incorrect commands will generally produce helpful error messages, although by default, the engine will terminate after getting an
incorrect command.
The easiest way to make the engine keep going after an incorrect command is to use the *debug* mode, either by typing `debug` or
passing `--debug` as command line flags.
This also turns on logging, which can be turned off again with `log off`.
Use `output <name>` to change how the engine prints the current position.
The default is `fen`, but it's also possible to generate ASCII or UTF-8 diagrams, or export the pgn:
For example, typing `show pgn` will keep the output unchanged but export a PGN of the current match.
`help`will print a short summary of additional commands, those commands wills generally produce context-dependent additional help in error
messages.

For example, to select the game *Chess* (this is already the default), type `play ataxx`.
To select the engine `GAPS` with eval `PiSTON`, type `engine gaps-piston`.
Names are case-insensitive; leaving out the eval will use the default eval for the current game,
which is `lite` for chess.
Alternatively, it's also possible to change the eval of an engine during the game without resetting the engine using `set-eval`
(the `eval` command instead prints the static eval of the current position).
There are many more options, this document is too short to list them all in detail.

### Command line flags

Command line flags function similarly to user input at runtime, but are a bit more restrictive in some cases.

### Thanks

Huge thanks to everyone who helped me learn more about engine programming! 

