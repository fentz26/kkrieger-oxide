# Farbrausch / .theprodukkt

The exported development data in `assets/kkrieger3383.kx` and the Rust
translations of `KDoc::Init`, compact numeric decoders, spline evaluation,
animation instructions, and integer randomness derive from Farbrausch's
published source:

https://github.com/farbrausch/fr_public/tree/8c8f82c2ae01133370e649bbc05916e7aceecadd/werkkzeug3_kkrieger

Copyright (c) 2002-2012, Dierk Ohlerich, Fabian Giesen, Thomas Mahlke,
Christoph Muetze, Tammo Hinrichs, Bastian Zuehlke.

The complete BSD-2-Clause conditions and disclaimer are preserved in
`farbrausch-LICENSE.txt`. Retain them in distributions containing derived code
or data. Newly written project-specific code is MIT licensed.

The Rust outline reader uses ttf-parser 0.25.1 under its MIT license, preserved
in `ttf-parser-LICENSE.txt`. System fonts are read from the target machine and
are not embedded or relicensed by this project.

Reference-only files and the independently unpacked original release remain
under `.reference/`, outside the distributed application. Original .kkrieger
content and authorship are credited to .theprodukkt; the beta archive's
freeware terms remain applicable to that archive and its contents.

Mesh transcendental functions use libm 0.2.16, a pure Rust math implementation.
Its MIT license and upstream notices are preserved in `libm-LICENSE.txt`.
