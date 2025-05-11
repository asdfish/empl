# empl
terminal music player

# Building
```sh
git clone https://github.com/asdfish/empl.git --depth 1
cd empl
cargo install --path .
```

# Configuration
Configuration is done in the [config.rs](./src/config.rs) file by creating a `struct` that implements the `Config` trait and then make the `SelectedConfig` type alias point to it.

# Dependencies
 - Linux: alsa
 - windows: wasapi
 - darwin: coreaudio
 - android: oboe
