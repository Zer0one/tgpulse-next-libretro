# Release license collection

`tools/package_libretro.py` collects license, copyright and notice files from
the locked Cargo dependency closure for the selected target, including build
dependencies and bundled C/C++ libraries. The package's `THIRD_PARTY.json`
records package versions, SPDX declarations, authors and included text paths.
Local workspace crates retain the root MIT license and the existing MAME/YMFM
BSD notices. Source copies live under `LICENSES`; packaged texts use `licenses`.

Four registry archives omit their license texts. These copies come from the
exact source revisions recorded in each archive's `.cargo_vcs_info.json`:

- codespan-reporting 0.11.1: [LICENSE](https://github.com/brendanzab/codespan/blob/fd389a13f5bb6d625b71e2e4694b26e127f393f9/LICENSE).
- naga 0.19.2: [MIT](https://github.com/gfx-rs/wgpu/blob/61d779d4d67111bcf34f4b3b8b59e0d2dc6147f4/LICENSE.MIT) and [Apache](https://github.com/gfx-rs/wgpu/blob/61d779d4d67111bcf34f4b3b8b59e0d2dc6147f4/LICENSE.APACHE).
- spirv 0.3.0+sdk-1.3.268.0: [LICENSE](https://github.com/gfx-rs/rspirv/blob/e91a5debc10c395b36db7721dfff6326d697a1e2/LICENSE).
- hexf-parse 0.2.1: its original Cargo manifest declares CC0-1.0; include that
  declaration and the [Creative Commons legal text](https://creativecommons.org/publicdomain/zero/1.0/legalcode.txt).

The Libretro API notice follows the SM2 reference's public API license copy.
Review this inventory when Cargo.lock changes; a missing dependency license
text fails package assembly rather than silently omitting it.
