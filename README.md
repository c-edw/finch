# Finch

#### What is it?
Finch is a CLI tool which allows you to automatically enhance an image library using Google's Vision API, by replacing each image with the highest resolution version found. It is still currently work-in-progress.

#### How can I use it?
- Clone this repository
- Run `cargo install`
- Run `finch -h` for a list of parameters

# What needs doing?
- ~~Cleanup~~ (done)
- ~~CLI implementation~~ (done)
- Parallelise API calls (Vision API calls can take a long time and massively slows down the speed of Finch)
- Publish to `crates.io`
