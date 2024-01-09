name = Fabian Gruber
matriculation number = 11908627

# Setup
all projects are implemented using Rust.
to compile the projects, you need to install Rust using e.g. [rustup](https://www.rust-lang.org/tools/install)
it should be nothing more than executing this command:
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
after that, you should be able to run each project from its directory.
below are more instrucions on how to run the individual projects, most have --help flags to show their arguments
```
cargo run -- --help
```

# Project 1
this program takes 1 reqired argument for the path to the log file and a option to specify which host should fail.
```
cargo run --release sampledb.log -f Alice,Bob
```
it will print all the required tasks to the terminal and create a image with the visualization.

# Project 2
this program takes 2 required arguments, first the number of nodes and second the node id starting from 0.
you can either run each node in a new terminal using
```
cargo run --release 2 0
```
and
```
cargo run --release 2 1
```
or use the shell script to run n nodes in the same terminal
```
./run.sh 2
```
to adjust times like T_fail see the constants in src/main.rs

# Project 3
this programm takes no arguments, it creates 4 images, ring.png to visualize the chord ring and 3 load barplots.
to see which bars belong to which node refer to the infos printed by the program.
```
cargo run --release
```

# Project 4
this program takes 1 required argument for the world_file and has a couple of options
-e, --epochs <epochs>  Number of epochs
-c, --cat              Cat enabled
-d, --dumb             Dumb mouse enabled
-v, --visualize        Visualize game output
-s, --skip <skip>      Skip showing number of epochs
-p, --plot             Plot stats

by default it will run for 100_000 epochs, not spawn a cat, use the improved mouse, not visualize, not skip showing epochs and not plot stats.
to show the game pass -v, to spawn a cat use -c, ... see above.
the command below will spawn a cat and plot the stats.

```
cargo run --release world_setup/world_empty.txt -cp
```

