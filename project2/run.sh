#!/bin/sh

# build
cargo build --release

# spawn $1 nodes
for (( i = 0; i < $1; i++ ))
do
  ./target/release/project2 $1 $i &
  pids[${i}]=$!
done

# wait for all nodes
for pid in ${pids[*]}; do
    wait $pid
done
