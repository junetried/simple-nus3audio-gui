# simple-nus3audio-gui
A tool written in Rust using [FLTK](https://www.fltk.org/), [Kira](https://github.com/tesselode/kira), [rodio](https://github.com/RustAudio/rodio), and [jam1garner's libnus3audio](https://github.com/jam1garner/libnus3audio).

## Why?
I got tired of using the [NUS3Audio Editor](https://gamebanana.com/tools/6927) via WINE. Replacing audio was tedious when I had to start from root for each file thanks to the odd file dialog, and the software overall was far from stable. You could trigger a dotnet exception by pressing Play too fast.

## Setup
Download [the latest release of simple-nus3audio-gui](https://github.com/junetried/simple-nus3audio-gui/releases/latest). The release archive contains a version of VGAudioCli pulled from [their AppVeyor](https://ci.appveyor.com/project/Thealexbarney/VGAudio/build/artifacts). Unfortunately, this service only hosts old builds for up to a month, and [the latest release from GitHub](https://github.com/Thealexbarney/VGAudio/releases/latest) is very old. Therefore, it is highly recommended to use the VGAudioCli build from this release even if you plan on building from source (unless you wish to build VGAudioCli from source as well).

You will also want [vgmstream](https://github.com/vgmstream/vgmstream/releases/latest). A working build of this is already included in the release archive. This program is not strictly necessary for the function of simple-nus3audio-gui, but will make decoding IDSP and LOPUS files faster (especially on non-Windows platforms using WINE), as well as allow you to retrieve loop data from existing files, so it is recommended to use.

The default settings will look for VGAudioCli and vgmstream in the same directory as they appear in the release archive. For convenience sake, you can keep these in the same directory, but you can change the location of either of these programs as soon as you run simple-nus3audio-gui for the first time.

Finally, run simple-nus3audio-gui. On first start, you'll be asked to download VGAudioCli if you haven't already and configure the path to it. If the defaults mentioned above are correct, you don't need to do anything. If you placed VGAudioCli in a different folder, configure the path before continuing.

### For non-Windows users
VGAudioCli is a command-line Windows-only tool using dotnet. To play the idsp-format files found in nus3audio files, or to import files of another type, VGAudioCli is required.

Fortunately, you can run the tool using Mono, dotnet (Microsoft's own runtime), or WINE. It is recommended to use Mono, as WINE's overhead adds a long delay each time VGAudioCli is run. On first start, the program will try to find `mono` or `dotnet` in the PATH before using `wine`. One of these three should be installed to run VGAudioCli.

### For MacOS users
I haven't tested this program on MacOS. With that in mind, this program should still work in MacOS, provided you have the prerequisite described above. If it doesn't, please open [an issue](https://github.com/junetried/simple-nus3audio-gui/issues) describing what went wrong.

## Building
Nothing special. Make sure you have the Rust compiler installed (try [rustup.rs](https://rustup.rs/)) and run `cargo build --release`.
