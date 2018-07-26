# Development Halted
Unfortunately, I have discovered some odd behaviour and instabilities in Google's Vision API which prevents Finch from doing the best job it can. They have been reported, but I have yet to hear anything about them being fixed. I don't recommend using Finch in its current state. It's still user-unfriendly and could have unexpected behavior. I will resume development on Finch once Google's API is stable.

# Finch 

#### What is it?
Are you an image hoarder? Couldn't resist saving that Zuckerberg meme, or cat chasing a laser beam? It's unlikely those images were in their original, high resolution glory.

Finch uses Google's Vision API to automatically reverse image search images in your collection. If it finds a higher resolution version, it will automatically download and replace the saved version!

#### How can I use it?
To use Finch, you'll need a Vision API key, which you can get by registering for Google Cloud Platform. They offer a free 1,000 Vision API calls per month.

- Clone this repository
- Run `cargo install`
- Run `finch -h` for a list of parameters

#### Supported platforms.
Finch has only been tested on GNU/Linux (Ubuntu 18.04). If it doesn't work for you, please submit an issue describing the problem in as much detail as possible.

#### Notes.
- Finch still has a fairly primitive user interface. It may not exit immediately if it encounters a problem, making unnecessary calls to the Vision API. 
- Finch makes calls in parallel rather than asynchronously. This can cause performance issues.
- It may work intermittently on slow connections.

#### Why does this (fairly) simple project have so many commits?
I am constantly rewriting Finch as I learn better practices, sometimes adding new features along the way.