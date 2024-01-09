# Name
Fabian Gruber

# Matriculation number
11908627

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

empty, without cat, dumb mouse: 
fed = 470
eaten = 0
time_to_cheese mean = 3648.6350364963487 +/- 587.0459812126829
fed per 1000 epochs mean = 0.2712712712712714 +/- 0.021327192106714587

empty, without cat, smart mouse:
fed = 2926
eaten = 0
time_to_cheese mean = 697.8120195667369 +/- 37.965461217926226
fed per 1000 epochs mean = 1.431431431431431 +/- 0.06033212800931284

empty, with cat, dumb mouse:
fed = 8548
eaten = 359070
time_to_cheese mean = 194.539580352885 +/- 3.7532429465524393
fed per 1000 epochs mean = 4.191191191191186 +/- 0.07778574578919932

empty, with cat, smart mouse:
fed = 9373
eaten = 370558
time_to_cheese mean = 176.9838322045006 +/- 3.235991809341996
fed per 1000 epochs mean = 4.571571571571566 +/- 0.0789005454369692

walls, without cat, dumb mouse: 

walls, without cat, smart mouse:

walls, with cat, dumb mouse:

walls, with cat, smart mouse:

