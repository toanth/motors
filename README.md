![image](/MotorsEngineChess.png)

# Motors

This repository contains 4 crates related to board games:

- `gears`: **Board representation** and general utility
- `motors`: **Engines**
- `monitors`: a WIP **UGI client**, will eventually include a GUI (text-based for now)
- `pliers`: **Tuner** for HCE eval weights

Currently, the most interesting part is probably the superhuman UCI [Chess960](<https://en.wikipedia.org/wiki/Fischer_random_chess>), DFRC
and **Chess Engine** `CAPS-LiTE`.

[//]: # (Motors &#40;**Mo**re **t**han **or**dinary **s**earchers&#41;)
Motors
is both the name of this GitHub repo,

and of the `motors` crate, which contains engines.

An engine typically consists of two parts, the *searcher* and the *evaluation function*,
such as `caps-lite`.
These parts can be freely changed, including during a match.

## Searchers

### CAPS

A chess searcher estimated at > 3k elo when paired with the hand-crafted evaluation function `LiTE`.

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
- Adjusts pruning and reduction margins based on eval difference to previous move
- Time Management with a soft and hard bound, as well as support for fixed time, depth, nodes, mate and `infinite`, or any combination of
  these
    - Almost full UCI compliance, including `searchmoves`, `ponder`, `multipv`, etc. Notable missing features are endgame tablebases and a
      built-in opening book.
- Eval function can be changed at runtime (see below)

### Other Searchers

In addition to the **C**hess-playing **A**lpha-beta **P**runing **S**earch (**CAPS**),
there is also the **G**eneral **A**lpha-beta **P**runing **S**earch (**GAPS**, a game-agnostic engine, currently still very basic),
and **Random** (a random mover).
Except for **Random**, those engines can be combined with any evaluation function supporting the current game.

They are currently being worked on and should become significantly stronger in the future.
Further plans include additional engines, like **M**inimalistic **A**lpha-beta **P**runing **S**earch (**MAPS**),
a simple alpha-beta pruning search without any further techniques, and an MCTS searcher.

## Evaluation Functions

### LiTE

**Li**near **T**uned **Eval**, a chess eval using a linear combination of weights which have
been tuned using the `pliers` tuner.
It can be interpreted as a single layers perceptron, a neural net consisting of a single neuron.
Such an eval functions is also often called a Hand-Crafted Eval function (HCE).
This is the default eval for chess.

### PiSTOn

**Pi**ece **S**quare **T**able **On**ly eval, a chess eval using only piece square tables,
similar to the well-known PeSTO engine.

### MateOnCE

**Mate**rial **On**ly **C**hess **E**val, a material-only evaluation function for chess,
using the classical piece values 1, 3, 3, 5, 9.

### BAtE

**B**asic **At**axx **E**val, a very simple material counting eval for Ataxx.

### BasE

**Bas**ic m,n,k **E**val, a simple hand-crafted eval for m,n,k games.

### LUTE

**L**inear **U**ltimate **T**ic-tac-toe **E**val, a simple hand-crafted eval for UTTT.

### Random

Returns random values. Still stronger than the *random* engine when used as eval function
for an actual engine like `caps` or `gaps`.

## Games

Currently, 4 games are implemented:

- **Chess**, including Chess960 (a.k.a. Fischer Random Chess) and Double Fischer Random Chess (DFRC)
- [**Ataxx**](https://en.wikipedia.org/wiki/Ataxx), a challenging board game where the goal is to convert your opponent's pieces
- [**m, n, k**](https://en.wikipedia.org/wiki/M,n,k-game) games, a generalization of Tic-Tac-Toe that can actually be difficult. The current
  implementation is somewhat limited and does
  not support boards larger than 128 squares, nor does it (yet) support rules specific to variants such as Connect 4 or Gomoku.
- [**Ultimate Tic-Tac-Toe**](https://en.wikipedia.org/wiki/Ultimate_tic-tac-toe), a much more challenging version of Tic-Tac-Toe where every
  square is itself a Tic-Tac-Toe board.

## Usage

### Building

To build the engines, it is enough to type `make` or use `cargo`.
`cargo` can also be used to build other parts, such as the match manager `monitors` or the tuner `pliers`.
Individual games and engines can be included or excluded from the build through cargo features, but the default is to build everything.
Alternatively, I'm planning to do a GitHub release soon, which will contain at least the engines binary.

### Running

Starting the `motors` executable without any command line options will start the default game, `chess`, with `CAPS`, the default engine for
chess,
and `LiTE`, the default eval for chess. Coincidentally, this is also the strongest and most developed engine and eval.
This engine can be used out of the box with any UCI chess GUI.

#### Manual User Input

All engines use the UCI or the very similar and mostly compatible UGI protocol for communicating with the GUI.

But this interface has also been designed to be easy to use for a human.
Incorrect commands will generally produce helpful error messages.
Typing the start of a command will list context-dependent autocompletion options.

Use `output <name>` to change how the engine prints the current position.

The default is `pretty`, a human-readably diagram of the current position, but it's also possible to generate alternative
ASCII or UTF-8 diagrams, or export the FEN or PGN of the current match:

For example, typing `show pgn` will keep the output unchanged but export a PGN of the current match.
To select the game *Chess* (this is already the default), type `play chess`.
To select the engine `GAPS` with eval `PiSTON`, type `engine gaps-piston`.
Names are case-insensitive; leaving out the eval will use the default eval for the current game,
which is `lite` for chess.
Alternatively, it's also possible to change the eval of an engine during the game without resetting the engine using `set_eval`
(the `eval` command instead prints the static eval of the current position).
There are many more options, this document is too short to list them all in detail.

### Command line flags

Command line flags are handled similarly to user input at runtime, but are a bit more restrictive in some cases.
For example, to play `Ataxx` with `GAPS` and the `BAtE` eval, pass the following command-line flags: `--game ataxx --engine gaps-bate`.
`bate` is already the default eval for `Ataxx`, and `GAPS` is the default engine for `Ataxx`, so this is equivalent to just `--game ataxx`.

## Thanks

Huge thanks to everyone who helped me learn more about engine programming! 

