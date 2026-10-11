This repository is a Lovense Connect maintenance fork of Intiface Central 3.2.2+46.

Base source: https://github.com/intiface/intiface-central/tree/5413bf0e23d22423c626b6bc63bd7673745a48c6

The app and its original copyrights and GPL-3.0 license are preserved. This is an unofficial fork, not a release from Nonpolynomial Labs. Intiface is their registered trademark.

The original published `buttplug_server_hwmgr_lovense_connect` 12.0.0 crate was downloaded from static.crates.io and verified against the SHA-256 checksum in the original rust/Cargo.lock before modification. It is retained under rust/vendor/lovense-connect with its BSD-3-Clause license. rust/Cargo.toml patches both the app and intiface-engine's dependency to this maintained copy, and pins intiface-engine 5.0.5.

See docs/lovense-connect-fork.md for connection and Windows build instructions.
