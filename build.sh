#!/bin/sh
# cd deps/
# rustc --crate-name deps export.rs --crate-type dylib --emit=dep-info,link -C opt-level=3 -C debuginfo=2 -C debug-assertions=on -C metadata=56d28e9b9c1fa4c1 --out-dir /home/tbm/programmering/rust/space_tennis/deps/target/debug/deps -L dependency=/home/tbm/programmering/rust/space_tennis/deps/target/debug/deps --extern glutin_window=/home/tbm/programmering/rust/space_tennis/deps/target/debug/deps/libglutin_window-0d29b39f927ec1a1.rlib --extern opengl_graphics=/home/tbm/programmering/rust/space_tennis/deps/target/debug/deps/libopengl_graphics-e0112749021ce8df.rlib --extern piston_window=/home/tbm/programmering/rust/space_tennis/deps/target/debug/deps/libpiston_window-f012fe30b6ec67eb.rlib
# cd -

# rustc --crate-name deps deps/export.rs --crate-type dylib --emit=dep-info,link -C prefer-dynamic \
#  -C debuginfo=2 -C metadata=56d28e9b9c1fa4c1 --out-dir /home/tbm/programmering/rust/space_tennis/target/debug/deps -L dependency=/home/tbm/programmering/rust/space_tennis/target/debug/deps --extern opengl_graphics=/home/tbm/programmering/rust/space_tennis/target/debug/deps/libopengl_graphics-2f9a5c2d882b462d.rlib --extern piston_window=/home/tbm/programmering/rust/space_tennis/target/debug/deps/libpiston_window-c28fa8b8514674a8.rlib --extern glutin_window=/home/tbm/programmering/rust/space_tennis/target/debug/deps/libglutin_window-e854e75fa1deb11c.rlib

# rustc --crate-name deps deps/export.rs --crate-type dylib --emit=dep-info,link -C prefer-dynamic \
#  -C debug-assertions=on -C opt-level=3 \
#  -C debuginfo=2 -C metadata=56d28e9b9c1fa4c1 --out-dir /home/tbm/programmering/rust/space_tennis/target/debug/deps -L dependency=/home/tbm/programmering/rust/space_tennis/target/debug/deps --extern opengl_graphics=/home/tbm/programmering/rust/space_tennis/target/debug/deps/libopengl_graphics-2f9a5c2d882b462d.rlib --extern piston_window=/home/tbm/programmering/rust/space_tennis/target/debug/deps/libpiston_window-c28fa8b8514674a8.rlib --extern glutin_window=/home/tbm/programmering/rust/space_tennis/target/debug/deps/libglutin_window-e854e75fa1deb11c.rlib
started=`date +%s`
# rustc --crate-name space_tennis space_tennis.rs --crate-type bin --emit=dep-info,link -C debuginfo=2 -C metadata=0e22ee5f4bfcdc6f -C extra-filename=-0e22ee5f4bfcdc6f --out-dir /home/tbm/programmering/rust/space_tennis/target/debug/deps -L dependency=/home/tbm/programmering/rust/space_tennis/target/debug/deps --extern deps=/home/tbm/programmering/rust/space_tennis/target/debug/deps/libdeps.so
rustc --crate-name space_tennis space_tennis.rs --crate-type bin --emit=dep-info,link -C debuginfo=2 -C metadata=0e22ee5f4bfcdc6f -C extra-filename=-0e22ee5f4bfcdc6f --out-dir /home/tbm/programmering/rust/space_tennis/target/debug -L dependency=/home/tbm/programmering/rust/space_tennis/target/debug/deps --extern deps=/home/tbm/programmering/rust/space_tennis/deps/target/debug/libdeps.so
exit=$?
ended=`date +%s`
if [ $exit -ne 0 ]; then
    exit $exit
fi
echo took $(($ended-$started)) seconds
LD_LIBRARY_PATH="/home/tbm/programmering/rust/space_tennis/target/debug/deps:/home/tbm/programmering/rust/space_tennis/target/debug/deps:/home/tbm/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib" \
  exec ./target/debug/space_tennis
