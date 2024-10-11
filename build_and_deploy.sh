cargo build --release
sleep .5
scp target/armv5te-unknown-linux-musleabi/release/Correctional robot@ev3dev.local:~/src