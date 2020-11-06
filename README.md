### Image Server Written In Rust
##### This is not production ready at all and mostly for personal use.
Http server that resizes images (or converts to webp) based on the query string. 
Images are optionally being cached in RAM to make serving as fast as possible.
Optionally set a whitelist to (probably) remove any security issues.
