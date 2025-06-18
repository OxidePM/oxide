
# Oxide

**Oxide** is a Package Manager based on Nix
**Oxide** is written in RUST and has a content-addressed store

Short term goals:
- [ ] Rewrite every recursive function to a non recursive version
to allow this PM to run on embedded systems, and to not pin futures
- [ ] Better error messages. With file and line number in debug mode
- [ ] Add GC
- [ ] Add Binary caches

Long term goals:
- [ ] Add support for multiple platforms. Right now only linux 
