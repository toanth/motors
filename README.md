# Motors

This repository contains 3 WIP crates related to board games:

- `gears`: **Board representation** and general utility
- `motors`: **Engines**
- `monitors`: a UGI client, including (eventually) a **GUI**

Currently, the most interesting part is probably the UCI **chess engine** `CAPS`.
It's very much in an early state right now,
but it should be fully functional and stronger than most humans.\
Current features:

- Alpha-beta Pruning Negamax
- Quiescent Search
- Move Ordering:
    - TT Move
    - MVV-LVA
    - Killer Move
    - Quiet History Heuristic
- Transposition Table
- Iterative Deepening
- Aspiration Windows
- Principal Variation Search
- Check Extensions
- Null Move Pruning
- Reverse Futlity Pruning
- Late Move Pruning
- Time Management with a soft and hard bound, as well as support for fixed time, depth, nodes and `infinite`, or any combination of these
- Eval function:
    - Piece Square Tables
    - Rooks and Kings on (Semi)Open/Closed Files
    - Passed Pawns
    - Separate Values for Middle- and Endgame, linearly interpolated
    - Values tuned using [this tuner](https://github.com/GediminasMasaitis/texel-tuner), using publicly available datasets

#### Thanks

Huge thanks to everyone who helped me learn more about engine programming! 

