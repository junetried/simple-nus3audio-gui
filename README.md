# simple-nus3audio-gui
A tool written in Rust using FLTK, rodio and [jam1garner's libnus3audio](https://github.com/jam1garner/libnus3audio).

## Why?
I got tired of using the [NUS3Audio Editor](https://gamebanana.com/tools/6927) via WINE. Replacing audio was tedious when I had to start from root for each file thanks to the odd file dialog, and the software overall was far from stable. You could trigger a dotnet exception by pressing Play too fast.

## Setup
Download the [latest release of simple-nus3audio-gui](https://github.com/EthanWeegee/simple-nus3audio-gui/releases/latest). Then, download the [latest release of VGAudioCli.exe](https://github.com/Thealexbarney/VGAudio/releases/latest). It is recommended that you place these in the same folder.

Run simple-nus3audio-gui. On first start, you'll be asked to download VGAudioCli and configure the path to it. If you placed VGAudioCli in a different folder, configure the path before continuing.

### For non-Windows users
VGAudioCli is a command-line Windows-only tool using dotnet. To play the idsp-format files found in nus3audio files, or to import files of another type, VGAudioCli is required.

Fortunately, you can run the tool using Mono, dotnet (Microsoft's own runtime), or WINE. It is recommended to use Mono, as WINE's overhead adds a long delay each time VGAudioCli is run. On first start, the program will try to find `mono` or `dotnet` in the PATH before using `wine`. One of these three should be installed to run VGAudioCli.

### For MacOS users
I haven't tested this program on MacOS. With that in mind, this program should still work in MacOS, provided you have the prerequisite described above. If it doesn't, please open [an issue](https://github.com/EthanWeegee/simple-nus3audio-gui/issues) describing what went wrong.

## Building
Nothing special. Make sure you have the Rust compiler installed (try [rustup.rs](https://rustup.rs/)) and run `cargo build --release`.
