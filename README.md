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
```
-e, --epochs <epochs>  Number of train epochs
-c, --cat              Cat enabled
-d, --dumb             Dumb mouse enabled
-v, --visualize        Visualize game output
-s, --skip <skip>      Skip showing number of epochs
-p, --plot             Plot stats
```

by default it will run for 100_000 epochs, not spawn a cat, use the improved mouse, not visualize, not skip showing epochs and not plot stats.
to show the game pass -v, to spawn a cat use -c, ... see above.
the command below will spawn a cat and plot the stats.

```
cargo run --release world_setup/world_empty.txt -cp
```

the following stats correspond to the provided images and were taken after 1_000_000 epochs of traning

empty, without cat, dumb mouse: 
- fed = 2452
- eaten = 0
- time_to_cheese mean = 928.3680297397779 +/- 148.70132644710114
- fed per 1000 epochs mean = 1.0730730730730713 +/- 0.06536416865798593

empty, without cat, smart mouse:
- fed = 58384
- eaten = 0
- time_to_cheese mean = 32.5795836131633 +/- 0.38870378239188386
- fed per 1000 epochs mean = 29.80280280280282 +/- 0.3749073720988146

empty, with cat, dumb mouse:
- fed = 27792
- eaten = 330637
- time_to_cheese mean = 51.167811212386226 +/- 0.4540805009029415
- fed per 1000 epochs mean = 16.02402402402402 +/- 0.13616537507572618

empty, with cat, smart mouse:
- fed = 40825
- eaten = 376225
- time_to_cheese mean = 35.55483987207794 +/- 0.24897337033042036
- fed per 1000 epochs mean = 22.2022022022022 +/- 0.15792379507362544

walls, without cat, dumb mouse: 
- fed = 1858
- eaten = 0
- time_to_cheese mean = 1714.2658662092633 +/- 130.36352283883036
- fed per 1000 epochs mean = 0.581581581581582 +/- 0.035677966690775005

walls, without cat, smart mouse:
- fed = 8837
- eaten = 0
- time_to_cheese mean = 179.34265103697084 +/- 11.378767640834004
- fed per 1000 epochs mean = 5.54954954954955 +/- 0.22706357652630343

walls, with cat, dumb mouse:
- fed = 585
- eaten = 9729
- time_to_cheese mean = 3363.8783783783765 +/- 515.2091427156209
- fed per 1000 epochs mean = 0.2952952952952958 +/- 0.028851054976219943

walls, with cat, smart mouse:
- fed = 19368
- eaten = 163671
- time_to_cheese mean = 89.63880390802369 +/- 1.646801640971177
- fed per 1000 epochs mean = 10.126126126126128 +/- 0.16208290523952368

