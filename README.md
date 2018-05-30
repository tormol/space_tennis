# Space tennis
A 3D pong game written in Rust using the [Piston](https://piston.rs) game engine.
![start](images/start.png)

## How to play / Game mechanics
The goal of the game is to block the ball from leaving the square tube at your side,
and make the computer-controlled oponent unable to do so at his end.  
![UI explanation](images/UI_explanation.png)
The ball bounces of walls without changing speed, but will take up a percentage
of the velocity of the racket when it hits it.  
A new round is started when somebody misses, and the loser starts with the
ball attached to his racket.  
Move your racket with the mouse, and click to start the round when you have the ball.
Pause the game when the ball is in motion by clicking, click again to unpause.

## Compiling
You need the Rust compiler and package manager, which can be downloaded from [rust-lang.org](https://rust-lang.org/en-US/install.html).
Then run these commands:
```sh
git clone https://github.com/tormol/space_tennis
cd space_tennis
cargo build --release
cargo run --release
```

## License
Copyright 2018 Torbjørn Birch Moltu. Licensed under the
GNU General Public License, as published by the Free Software Foundation,
either version 3 of the License, or (at your option) any later version.
