# Grain Processor
This crate performs grain extraction and analysis on a corpus of audio files. It stores the grain metadata, including analysis data, in a SQLite database. This data can be used for algorithmic granular synthesis. It is written in Rust for better performance, since one might conceivably wish to granulate gigabytes of audio files.

The idea is that we work through each file by extracting a grain, analyzing it, and moving forward *n* samples to extract the next grain. The grains are analyzed for various features, including spectral entropy, spectral flatness, spectral slope, etc.

## Configuration
There is a configuration file called `config.json` in the root of this repository that allows you to specify parameters for the program, such as where the audio files are located and how large the grains should be. You can specify multiple grain profiles in this configuration file. Each grain profile specifies the grain size in frames, and the distance between grain onsets for extraction. The extractor will extract grains separately for each profile. This is useful if you want grains of multiple sizes in your database, or if you're interested in trying different grain spacings. Place the configuration file in the same directory as the grain processor executable.

## Building
To build this crate, run `cargo build --release` from the root of the repository.

## Running
The grain processor may take some time to load all of the audio files. It will split longer files into smaller chunks to allow for faster multithreaded processing. The chunk size in frames is specified in the configuration file.

The configuration file also allows you to limit the number of concurrent threads. This option could be useful if you find that the grain processor is bogging down your CPU and preventing you from doing other things on your computer at the same time. If you don't care how many threads are running, you can just set this number to 0, and the number of threads will be determined automatically.

The grain processor will automatically create the database if it does not exist, but if the database exists, it will simply add to it rather than replacing the records that are already there. This allows you to grow your database over time by running the grain processor again and again with new audio.
