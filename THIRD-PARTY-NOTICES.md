# Tredjepartslicenser

Motorn och spel byggda med den distribuerar följande komponenter. Deras
licenser gäller oberoende av LGJT License v1, som bara täcker den här
motorns egen kod. De flesta (MIT, Apache-2.0, BSD) kräver att
upphovsrättsnotisen följer med i binärdistributioner – det gäller alltså
även exporterade spel.

Två saker värda att notera särskilt:

- **QuickJS** (MIT) är inbakad i `script_host.wasm` och följer därför med i
  varje exporterat spel.
- **zstd** (BSD-3-Clause / GPL-2.0 dual) används för .pak-arkiv. BSD-grenen
  gäller vid normal användning.

Fullständiga licenstexter finns i respektive paket under
`~/.cargo/registry/src/`, eller i paketets repo.

Listan genereras med `cargo metadata` – kör om den när beroenden ändras.

| Paket | Version | Licens |
|---|---|---|
| ab_glyph | 0.2.32 | Apache-2.0 |
| ab_glyph_rasterizer | 0.1.10 | Apache-2.0 |
| accesskit | 0.24.1 | MIT OR Apache-2.0 |
| addr2line | 0.26.1 | Apache-2.0 OR MIT |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| ahash | 0.8.12 | MIT OR Apache-2.0 |
| aho-corasick | 1.1.5 | Unlicense OR MIT |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 |
| ambient-authority | 0.0.2 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| android-activity | 0.6.1 | MIT OR Apache-2.0 |
| android-properties | 0.2.2 | MIT |
| android_system_properties | 0.1.6 | MIT OR Apache-2.0 |
| anstream | 1.0.0 | MIT OR Apache-2.0 |
| anstyle | 1.0.14 | MIT OR Apache-2.0 |
| anstyle-parse | 1.0.0 | MIT OR Apache-2.0 |
| anstyle-query | 1.1.5 | MIT OR Apache-2.0 |
| anstyle-wincon | 3.0.11 | MIT OR Apache-2.0 |
| anyhow | 1.0.104 | MIT OR Apache-2.0 |
| arbitrary | 1.4.2 | MIT OR Apache-2.0 |
| arboard | 3.6.1 | MIT OR Apache-2.0 |
| arrayref | 0.3.9 | BSD-2-Clause |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |
| as-raw-xcb-connection | 1.0.1 | MIT OR Apache-2.0 |
| ash | 0.38.0+1.3.281 | MIT OR Apache-2.0 |
| assert_type_match | 0.1.1 | MIT OR Apache-2.0 |
| async-executor | 1.14.0 | Apache-2.0 OR MIT |
| async-task | 4.7.1 | Apache-2.0 OR MIT |
| async-trait | 0.1.92 | MIT OR Apache-2.0 |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT |
| autocfg | 1.5.1 | Apache-2.0 OR MIT |
| base64 | 0.13.1 | MIT/Apache-2.0 |
| base64 | 0.22.1 | MIT OR Apache-2.0 |
| base64 | 0.23.1 | MIT OR Apache-2.0 |
| base64-simd | 0.8.0 | MIT |
| bevy_ecs | 0.19.1 | MIT OR Apache-2.0 |
| bevy_ecs_macro_logic | 0.19.1 | MIT OR Apache-2.0 |
| bevy_ecs_macros | 0.19.1 | MIT OR Apache-2.0 |
| bevy_macro_utils | 0.19.1 | MIT OR Apache-2.0 |
| bevy_platform | 0.19.1 | MIT OR Apache-2.0 |
| bevy_ptr | 0.19.1 | MIT OR Apache-2.0 |
| bevy_reflect | 0.19.1 | MIT OR Apache-2.0 |
| bevy_reflect_derive | 0.19.1 | MIT OR Apache-2.0 |
| bevy_tasks | 0.19.1 | MIT OR Apache-2.0 |
| bevy_utils | 0.19.1 | MIT OR Apache-2.0 |
| bit-set | 0.10.0 | Apache-2.0 OR MIT |
| bit-vec | 0.9.1 | Apache-2.0 OR MIT |
| bitflags | 1.3.2 | MIT/Apache-2.0 |
| bitflags | 2.13.1 | MIT OR Apache-2.0 |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| block2 | 0.5.1 | MIT |
| block2 | 0.6.2 | MIT |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 |
| bytecount | 0.6.9 | Apache-2.0/MIT |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| bytemuck_derive | 1.12.0 | Zlib OR Apache-2.0 OR MIT |
| byteorder | 1.5.0 | Unlicense OR MIT |
| byteorder-lite | 0.1.0 | Unlicense OR MIT |
| bytes | 1.12.1 | MIT |
| calloop | 0.13.0 | MIT |
| calloop | 0.14.4 | MIT |
| calloop-wayland-source | 0.3.0 | MIT |
| calloop-wayland-source | 0.4.1 | MIT |
| cap-primitives | 4.0.3 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| castaway | 0.2.4 | MIT |
| cc | 1.4.5 | MIT OR Apache-2.0 |
| cfg-if | 1.0.4 | MIT OR Apache-2.0 |
| cfg_aliases | 0.2.2 | MIT |
| chacha20 | 0.10.2 | MIT OR Apache-2.0 |
| clipboard-win | 5.4.1 | BSL-1.0 |
| cobs | 0.3.0 | MIT OR Apache-2.0 |
| codespan-reporting | 0.13.1 | Apache-2.0 |
| color | 0.3.3 | Apache-2.0 OR MIT |
| colorchoice | 1.0.5 | MIT OR Apache-2.0 |
| combine | 4.6.8 | MIT |
| compact_str | 0.10.0 | MIT |
| concurrent-queue | 2.5.0 | Apache-2.0 OR MIT |
| convert_case | 0.10.0 | MIT |
| core-foundation | 0.9.4 | MIT OR Apache-2.0 |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 |
| core-graphics | 0.23.2 | MIT OR Apache-2.0 |
| core-graphics-types | 0.1.3 | MIT OR Apache-2.0 |
| core_detect | 1.0.0 | MIT/Apache-2.0 |
| cow-utils | 0.1.3 | MIT |
| cpp_demangle | 0.5.1 | MIT OR Apache-2.0 |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| cpufeatures | 0.3.1 | MIT OR Apache-2.0 |
| cranelift-assembler-x64 | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-assembler-x64-meta | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-bforest | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-bitset | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-codegen | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-codegen-meta | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-codegen-shared | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-control | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-entity | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-frontend | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-isle | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-native | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| cranelift-srcgen | 0.135.2 | Apache-2.0 WITH LLVM-exception |
| crc32fast | 1.5.1 | MIT OR Apache-2.0 |
| critical-section | 1.2.0 | MIT OR Apache-2.0 |
| crossbeam-deque | 0.8.8 | MIT OR Apache-2.0 |
| crossbeam-epoch | 0.9.21 | MIT OR Apache-2.0 |
| crossbeam-queue | 0.3.14 | MIT OR Apache-2.0 |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 |
| crunchy | 0.2.4 | MIT |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| cursor-icon | 1.2.0 | MIT OR Apache-2.0 OR Zlib |
| debugid | 0.8.0 | Apache-2.0 |
| defmt | 1.1.1 | MIT OR Apache-2.0 |
| defmt-macros | 1.1.1 | MIT OR Apache-2.0 |
| defmt-parser | 1.0.0 | MIT OR Apache-2.0 |
| derive_more | 2.1.1 | MIT |
| derive_more-impl | 2.1.1 | MIT |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| directories-next | 2.0.0 | MIT OR Apache-2.0 |
| dirs-sys-next | 0.1.2 | MIT OR Apache-2.0 |
| dispatch | 0.2.0 | MIT |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 |
| disqualified | 1.0.0 | MIT OR Apache-2.0 |
| dlib | 0.5.3 | MIT |
| document-features | 0.2.12 | MIT OR Apache-2.0 |
| downcast-rs | 1.2.1 | MIT/Apache-2.0 |
| downcast-rs | 2.0.2 | MIT OR Apache-2.0 |
| dpi | 0.1.2 | Apache-2.0 AND MIT |
| dragonbox_ecma | 0.1.12 | Apache-2.0 WITH LLVM-exception OR BSL-1.0 |
| ecolor | 0.36.2 | MIT OR Apache-2.0 |
| egui | 0.36.2 | MIT OR Apache-2.0 |
| egui-wgpu | 0.36.2 | MIT OR Apache-2.0 |
| egui-winit | 0.36.2 | MIT OR Apache-2.0 |
| either | 1.18.0 | MIT OR Apache-2.0 |
| emath | 0.36.2 | MIT OR Apache-2.0 |
| embedded-io | 0.4.0 | MIT OR Apache-2.0 |
| embedded-io | 0.6.1 | MIT OR Apache-2.0 |
| encoding_rs | 0.8.41 | (Apache-2.0 OR MIT) AND BSD-3-Clause |
| env_filter | 2.0.0 | MIT OR Apache-2.0 |
| env_logger | 0.11.11 | MIT OR Apache-2.0 |
| epaint | 0.36.2 | MIT OR Apache-2.0 |
| epaint_default_fonts | 0.36.2 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 |
| equivalent | 1.0.2 | Apache-2.0 OR MIT |
| erased-serde | 0.4.10 | MIT OR Apache-2.0 |
| errno | 0.3.14 | MIT OR Apache-2.0 |
| error-code | 3.4.0 | BSL-1.0 |
| euclid | 0.22.14 | MIT OR Apache-2.0 |
| fastrand | 2.5.0 | Apache-2.0 OR MIT |
| fax | 0.2.7 | MIT |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 |
| fearless_simd | 0.4.1 | Apache-2.0 OR MIT |
| find-msvc-tools | 0.1.12 | MIT OR Apache-2.0 |
| fixedbitset | 0.4.2 | MIT/Apache-2.0 |
| fixedbitset | 0.5.7 | MIT OR Apache-2.0 |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |
| fnv | 1.0.7 | Apache-2.0 / MIT |
| foldhash | 0.2.0 | Zlib |
| font-types | 0.12.5 | MIT OR Apache-2.0 |
| foreign-types | 0.5.0 | MIT/Apache-2.0 |
| foreign-types-macros | 0.2.4 | MIT/Apache-2.0 |
| foreign-types-shared | 0.3.1 | MIT/Apache-2.0 |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 |
| fs-set-times | 0.20.3 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| futures | 0.3.34 | MIT OR Apache-2.0 |
| futures-channel | 0.3.34 | MIT OR Apache-2.0 |
| futures-core | 0.3.34 | MIT OR Apache-2.0 |
| futures-io | 0.3.34 | MIT OR Apache-2.0 |
| futures-lite | 2.6.1 | Apache-2.0 OR MIT |
| futures-sink | 0.3.34 | MIT OR Apache-2.0 |
| futures-task | 0.3.34 | MIT OR Apache-2.0 |
| futures-util | 0.3.34 | MIT OR Apache-2.0 |
| fxprof-processed-profile | 0.8.1 | MIT OR Apache-2.0 |
| generic-array | 0.14.7 | MIT |
| gethostname | 1.1.0 | Apache-2.0 |
| getrandom | 0.2.17 | MIT OR Apache-2.0 |
| getrandom | 0.3.4 | MIT OR Apache-2.0 |
| getrandom | 0.4.3 | MIT OR Apache-2.0 |
| gimli | 0.33.0 | MIT OR Apache-2.0 |
| gl_generator | 0.14.0 | Apache-2.0 |
| glam | 0.32.1 | MIT OR Apache-2.0 |
| glam | 0.33.7 | MIT OR Apache-2.0 |
| glifo | 0.2.0 | Apache-2.0 OR MIT |
| glow | 0.17.0 | MIT OR Apache-2.0 OR Zlib |
| gltf | 1.4.1 | MIT OR Apache-2.0 |
| gltf-derive | 1.4.1 | MIT OR Apache-2.0 |
| gltf-json | 1.4.1 | MIT OR Apache-2.0 |
| glutin_wgl_sys | 0.6.1 | Apache-2.0 |
| gpu-allocator | 0.28.0 | MIT OR Apache-2.0 |
| guillotiere | 0.7.0 | MIT/Apache-2.0 |
| half | 2.7.1 | MIT OR Apache-2.0 |
| harfrust | 0.12.0 | MIT |
| hash32 | 0.3.1 | MIT OR Apache-2.0 |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| heapless | 0.9.3 | MIT OR Apache-2.0 |
| heck | 0.5.0 | MIT OR Apache-2.0 |
| hermit-abi | 0.5.3 | MIT OR Apache-2.0 |
| hmac-sha1-compact | 1.1.7 | ISC |
| icu_collections | 2.3.0 | Unicode-3.0 |
| icu_locale_core | 2.3.0 | Unicode-3.0 |
| icu_locale_fallback | 2.3.0 | Unicode-3.0 |
| icu_locale_fallback_data | 2.3.0 | Unicode-3.0 |
| icu_normalizer | 2.3.0 | Unicode-3.0 |
| icu_normalizer_data | 2.3.0 | Unicode-3.0 |
| icu_properties | 2.3.0 | Unicode-3.0 |
| icu_properties_data | 2.3.0 | Unicode-3.0 |
| icu_provider | 2.3.1 | Unicode-3.0 |
| icu_segmenter | 2.3.0 | Unicode-3.0 |
| icu_segmenter_data | 2.3.0 | Unicode-3.0 |
| id-arena | 2.3.0 | MIT/Apache-2.0 |
| idna | 1.1.0 | MIT OR Apache-2.0 |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT |
| image | 0.25.10 | MIT OR Apache-2.0 |
| indexmap | 2.14.2 | Apache-2.0 OR MIT |
| inflections | 1.1.1 | MIT |
| io-extras | 0.19.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| io-lifetimes | 2.0.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| io-lifetimes | 3.0.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| ipnet | 2.12.2 | MIT OR Apache-2.0 |
| is_terminal_polyfill | 1.70.2 | MIT OR Apache-2.0 |
| itertools | 0.14.0 | MIT OR Apache-2.0 |
| itertools | 0.15.0 | MIT OR Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| ittapi | 0.4.0 | GPL-2.0-only OR BSD-3-Clause |
| ittapi-sys | 0.4.0 | GPL-2.0-only OR BSD-3-Clause |
| jiff | 0.2.35 | Unlicense OR MIT |
| jiff-core | 0.1.0 | Unlicense OR MIT |
| jiff-static | 0.2.35 | Unlicense OR MIT |
| jni | 0.22.4 | MIT OR Apache-2.0 |
| jni-macros | 0.22.4 | MIT OR Apache-2.0 |
| jni-sys | 0.3.1 | MIT OR Apache-2.0 |
| jni-sys | 0.4.1 | MIT OR Apache-2.0 |
| jni-sys-macros | 0.4.1 | MIT OR Apache-2.0 |
| jobserver | 0.1.35 | MIT OR Apache-2.0 |
| js-sys | 0.3.105 | MIT OR Apache-2.0 |
| json-escape-simd | 3.1.1 | MIT |
| khronos-egl | 6.0.0 | MIT/Apache-2.0 |
| khronos_api | 3.1.0 | Apache-2.0 |
| kurbo | 0.13.1 | Apache-2.0 OR MIT |
| lazy_static | 1.5.0 | MIT OR Apache-2.0 |
| leb128 | 0.2.7 | MIT OR Apache-2.0 |
| leb128fmt | 0.1.0 | MIT OR Apache-2.0 |
| libc | 0.2.189 | MIT OR Apache-2.0 |
| libloading | 0.8.9 | ISC |
| libm | 0.2.16 | MIT |
| libredox | 0.1.23 | MIT |
| linebender_resource_handle | 0.1.1 | Apache-2.0 OR MIT |
| linux-raw-sys | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| litemap | 0.8.3 | Unicode-3.0 |
| litrs | 1.0.0 | MIT OR Apache-2.0 |
| lock_api | 0.4.14 | MIT OR Apache-2.0 |
| log | 0.4.34 | MIT OR Apache-2.0 |
| mach2 | 0.6.0 | BSD-2-Clause OR MIT OR Apache-2.0 |
| maybe-owned | 0.3.4 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| memfd | 0.6.6 | MIT OR Apache-2.0 |
| memmap2 | 0.9.11 | MIT OR Apache-2.0 |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| mio | 1.2.3 | MIT |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 |
| multiversion | 0.9.0 | MIT OR Apache-2.0 |
| multiversion-macros | 0.9.0 | MIT OR Apache-2.0 |
| multiversion_no_op | 1.0.0 | Apache-2.0 OR MIT |
| naga | 30.0.1 | MIT OR Apache-2.0 |
| naga-types | 30.0.1 | MIT OR Apache-2.0 |
| ndk | 0.9.0 | MIT OR Apache-2.0 |
| ndk-context | 0.1.1 | MIT OR Apache-2.0 |
| ndk-sys | 0.6.0+11769913 | MIT OR Apache-2.0 |
| nohash-hasher | 0.2.0 | Apache-2.0 OR MIT |
| nonmax | 0.5.5 | MIT OR Apache-2.0 |
| num-bigint | 0.5.1 | MIT OR Apache-2.0 |
| num-integer | 0.1.47 | MIT OR Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| num_enum | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 |
| num_enum_derive | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 |
| objc-sys | 0.3.5 | MIT |
| objc2 | 0.5.2 | MIT |
| objc2 | 0.6.4 | MIT |
| objc2-app-kit | 0.2.2 | MIT |
| objc2-app-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-cloud-kit | 0.2.2 | MIT |
| objc2-contacts | 0.2.2 | MIT |
| objc2-core-data | 0.2.2 | MIT |
| objc2-core-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-core-graphics | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-core-image | 0.2.2 | MIT |
| objc2-core-location | 0.2.2 | MIT |
| objc2-encode | 4.1.0 | MIT |
| objc2-foundation | 0.2.2 | MIT |
| objc2-foundation | 0.3.2 | MIT |
| objc2-io-surface | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-link-presentation | 0.2.2 | MIT |
| objc2-metal | 0.2.2 | MIT |
| objc2-metal | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-quartz-core | 0.2.2 | MIT |
| objc2-quartz-core | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-symbols | 0.2.2 | MIT |
| objc2-ui-kit | 0.2.2 | MIT |
| objc2-ui-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-uniform-type-identifiers | 0.2.2 | MIT |
| objc2-user-notifications | 0.2.2 | MIT |
| object | 0.39.1 | Apache-2.0 OR MIT |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |
| once_cell_polyfill | 1.70.2 | MIT OR Apache-2.0 |
| orbclient | 0.3.55 | MIT |
| ordered-float | 5.5.0 | MIT |
| outref | 0.5.2 | MIT |
| owned_ttf_parser | 0.25.1 | Apache-2.0 |
| oxc | 0.149.0 | MIT |
| oxc-browserslist | 5.0.1 | MIT |
| oxc_allocator | 0.149.0 | MIT |
| oxc_ast | 0.149.0 | MIT |
| oxc_ast_macros | 0.149.0 | MIT |
| oxc_ast_visit | 0.149.0 | MIT |
| oxc_codegen | 0.149.0 | MIT |
| oxc_compat | 0.149.0 | MIT |
| oxc_data_structures | 0.149.0 | MIT |
| oxc_diagnostics | 0.149.0 | MIT |
| oxc_ecmascript | 0.149.0 | MIT |
| oxc_estree | 0.149.0 | MIT |
| oxc_index | 5.0.0 | MIT |
| oxc_parser | 0.149.0 | MIT |
| oxc_regular_expression | 0.149.0 | MIT |
| oxc_semantic | 0.149.0 | MIT |
| oxc_sourcemap | 8.1.2 | BSD-3-Clause |
| oxc_span | 0.149.0 | MIT |
| oxc_str | 0.149.0 | MIT |
| oxc_syntax | 0.149.0 | MIT |
| oxc_transformer | 0.149.0 | MIT |
| oxc_transformer_plugins | 0.149.0 | MIT |
| oxc_traverse | 0.149.0 | MIT |
| parking | 2.2.1 | Apache-2.0 OR MIT |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 |
| peniko | 0.6.1 | Apache-2.0 OR MIT |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 |
| petgraph | 0.6.5 | MIT OR Apache-2.0 |
| phf | 0.14.0 | MIT |
| phf_generator | 0.14.0 | MIT |
| phf_macros | 0.14.0 | MIT |
| phf_shared | 0.14.0 | MIT |
| pin-project | 1.1.13 | Apache-2.0 OR MIT |
| pin-project-internal | 1.1.13 | Apache-2.0 OR MIT |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 |
| plain | 0.2.3 | MIT/Apache-2.0 |
| png | 0.18.1 | MIT OR Apache-2.0 |
| polling | 3.11.0 | Apache-2.0 OR MIT |
| pollster | 1.0.1 | Apache-2.0/MIT |
| polycool | 0.4.0 | MIT OR Apache-2.0 |
| portable-atomic | 1.15.0 | Apache-2.0 OR MIT |
| portable-atomic-util | 0.2.8 | Apache-2.0 OR MIT |
| postcard | 1.1.3 | MIT OR Apache-2.0 |
| potential_utf | 0.1.6 | Unicode-3.0 |
| presser | 0.3.1 | MIT OR Apache-2.0 |
| proc-macro-crate | 3.5.0 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| profiling | 1.0.18 | MIT OR Apache-2.0 |
| pulley-interpreter | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| pulley-macros | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 |
| quick-error | 2.0.1 | MIT/Apache-2.0 |
| quick-xml | 0.41.0 | MIT |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later |
| rand | 0.10.2 | MIT OR Apache-2.0 |
| rand_core | 0.10.1 | MIT OR Apache-2.0 |
| range-alloc | 0.1.5 | MIT OR Apache-2.0 |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib |
| raw-window-metal | 1.1.0 | MIT OR Apache-2.0 |
| rayon | 1.12.0 | MIT OR Apache-2.0 |
| rayon-core | 1.13.0 | MIT OR Apache-2.0 |
| read-fonts | 0.41.0 | MIT OR Apache-2.0 |
| redox_syscall | 0.4.1 | MIT |
| redox_syscall | 0.5.18 | MIT |
| redox_syscall | 0.9.4 | MIT |
| redox_users | 0.4.6 | MIT |
| regalloc2 | 0.15.2 | Apache-2.0 WITH LLVM-exception |
| regex | 1.13.1 | MIT OR Apache-2.0 |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 |
| renderdoc-sys | 1.1.0 | MIT OR Apache-2.0 |
| ron | 0.12.2 | MIT OR Apache-2.0 |
| ropey | 1.6.1 | MIT |
| rustc-demangle | 0.1.28 | MIT/Apache-2.0 |
| rustc-hash | 1.1.0 | Apache-2.0/MIT |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT |
| rustc_version | 0.4.1 | MIT OR Apache-2.0 |
| rustix | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| rustix | 1.1.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| rustix-linux-procfs | 0.1.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| rustversion | 1.0.23 | MIT OR Apache-2.0 |
| same-file | 1.0.6 | Unlicense/MIT |
| scoped-tls | 1.0.1 | MIT/Apache-2.0 |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |
| sctk-adwaita | 0.10.1 | MIT |
| self_cell | 1.3.0 | Apache-2.0 OR GPL-2.0-only |
| semver | 1.0.28 | MIT OR Apache-2.0 |
| seq-macro | 0.3.6 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.151 | MIT OR Apache-2.0 |
| serde_spanned | 1.1.1 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| shlex | 2.0.1 | MIT OR Apache-2.0 |
| simd-adler32 | 0.3.10 | MIT |
| simd_cesu8 | 1.2.0 | Apache-2.0 OR MIT |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 |
| siphasher | 1.0.3 | MIT/Apache-2.0 |
| skrifa | 0.44.0 | MIT OR Apache-2.0 |
| slab | 0.4.12 | MIT |
| slotmap | 1.1.1 | Zlib |
| smallvec | 1.16.0 | MIT OR Apache-2.0 |
| smawk | 0.3.3 | MIT |
| smithay-client-toolkit | 0.19.2 | MIT |
| smithay-client-toolkit | 0.20.0 | MIT |
| smithay-clipboard | 0.7.3 | MIT |
| smol_str | 0.2.2 | MIT OR Apache-2.0 |
| socket2 | 0.6.5 | MIT OR Apache-2.0 |
| spin | 0.10.1 | MIT |
| spirv | 0.4.0+sdk-1.4.341.0 | Apache-2.0 |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 |
| str_indices | 0.4.4 | MIT OR Apache-2.0 |
| strict-num | 0.1.1 | MIT |
| syn | 2.0.119 | MIT OR Apache-2.0 |
| syn | 3.0.5 | MIT OR Apache-2.0 |
| synstructure | 0.13.2 | MIT |
| target-lexicon | 0.13.5 | Apache-2.0 WITH LLVM-exception |
| tempfile | 3.27.0 | MIT OR Apache-2.0 |
| termcolor | 1.4.1 | Unlicense OR MIT |
| textwrap | 0.16.3 | MIT |
| thiserror | 1.0.69 | MIT OR Apache-2.0 |
| thiserror | 2.0.20 | MIT OR Apache-2.0 |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 |
| thiserror-impl | 2.0.20 | MIT OR Apache-2.0 |
| thread_local | 1.1.10 | MIT OR Apache-2.0 |
| tiff | 0.11.3 | MIT |
| tiny-skia | 0.11.4 | BSD-3-Clause |
| tiny-skia-path | 0.11.4 | BSD-3-Clause |
| tinystr | 0.8.4 | Unicode-3.0 |
| tokio | 1.53.1 | MIT |
| toml | 0.9.12+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_datetime | 0.7.5+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_datetime | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_edit | 0.25.13+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_parser | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_writer | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 |
| tracing | 0.1.44 | MIT |
| tracing-attributes | 0.1.31 | MIT |
| tracing-core | 0.1.36 | MIT |
| ttf-parser | 0.25.1 | MIT OR Apache-2.0 |
| type-map | 0.5.1 | MIT/Apache-2.0 |
| typeid | 1.0.3 | MIT OR Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicode-general-category | 1.1.0 | Apache-2.0 |
| unicode-id-start | 1.4.0 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 |
| unicode-xid | 0.2.6 | MIT OR Apache-2.0 |
| url | 2.5.8 | MIT OR Apache-2.0 |
| urlencoding | 2.1.3 | MIT |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT |
| utf8parse | 0.2.2 | Apache-2.0 OR MIT |
| uuid | 1.26.0 | Apache-2.0 OR MIT |
| variadics_please | 1.1.0 | MIT OR Apache-2.0 |
| vello_common | 0.1.0 | Apache-2.0 OR MIT |
| vello_cpu | 0.1.0 | Apache-2.0 OR MIT |
| version_check | 0.9.5 | MIT/Apache-2.0 |
| vsimd | 0.8.0 | MIT |
| walkdir | 2.5.0 | Unlicense/MIT |
| wasi | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasip2 | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasm-bindgen | 0.2.128 | MIT OR Apache-2.0 |
| wasm-bindgen-futures | 0.4.78 | MIT OR Apache-2.0 |
| wasm-bindgen-macro | 0.2.128 | MIT OR Apache-2.0 |
| wasm-bindgen-macro-support | 0.2.128 | MIT OR Apache-2.0 |
| wasm-bindgen-shared | 0.2.128 | MIT OR Apache-2.0 |
| wasm-compose | 0.254.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasm-encoder | 0.254.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasm-encoder | 0.259.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasm-metadata | 0.254.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasmparser | 0.254.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasmparser | 0.259.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasmprinter | 0.254.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasmtime | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-environ | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-cache | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-component-macro | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-component-util | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-core | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-cranelift | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-fiber | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-jit-debug | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-jit-icache-coherence | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-unwinder | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-versioned-export-macros | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-internal-wit-bindgen | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-wasi | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wasmtime-wasi-io | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wast | 35.0.2 | Apache-2.0 WITH LLVM-exception |
| wast | 259.0.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wat | 1.259.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wayland-backend | 0.3.17 | MIT |
| wayland-client | 0.31.15 | MIT |
| wayland-csd-frame | 0.3.0 | MIT |
| wayland-cursor | 0.31.14 | MIT |
| wayland-protocols | 0.32.13 | MIT |
| wayland-protocols-experimental | 20250721.0.1 | MIT |
| wayland-protocols-misc | 0.3.12 | MIT |
| wayland-protocols-plasma | 0.3.12 | MIT |
| wayland-protocols-wlr | 0.3.12 | MIT |
| wayland-scanner | 0.31.11 | MIT |
| wayland-sys | 0.31.11 | MIT |
| web-sys | 0.3.105 | MIT OR Apache-2.0 |
| web-task | 1.1.3 | MIT OR Apache-2.0 |
| web-time | 1.1.0 | MIT OR Apache-2.0 |
| webbrowser | 1.2.4 | MIT OR Apache-2.0 |
| weezl | 0.1.12 | MIT OR Apache-2.0 |
| wgpu | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core-deps-apple | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core-deps-emscripten | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core-deps-wasm | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core-deps-windows-linux-android | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-hal | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-naga-bridge | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-types | 29.0.4 | MIT OR Apache-2.0 |
| wgpu-types | 30.0.1 | MIT OR Apache-2.0 |
| wiggle | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wiggle-generate | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| wiggle-macro | 48.0.2 | Apache-2.0 WITH LLVM-exception |
| winapi | 0.3.9 | MIT/Apache-2.0 |
| winapi-i686-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 |
| winapi-util | 0.1.11 | Unlicense OR MIT |
| winapi-x86_64-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 |
| windows | 0.62.2 | MIT OR Apache-2.0 |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 |
| windows-core | 0.62.2 | MIT OR Apache-2.0 |
| windows-future | 0.3.2 | MIT OR Apache-2.0 |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 |
| windows-result | 0.4.1 | MIT OR Apache-2.0 |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 |
| windows-sys | 0.59.0 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 |
| windows_aarch64_gnullvm | 0.52.6 | MIT OR Apache-2.0 |
| windows_aarch64_msvc | 0.52.6 | MIT OR Apache-2.0 |
| windows_i686_gnu | 0.52.6 | MIT OR Apache-2.0 |
| windows_i686_gnullvm | 0.52.6 | MIT OR Apache-2.0 |
| windows_i686_msvc | 0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_gnu | 0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 |
| winit | 0.30.13 | Apache-2.0 |
| winnow | 0.7.15 | MIT |
| winnow | 1.0.4 | MIT |
| winx | 0.36.4 | Apache-2.0 WITH LLVM-exception |
| wit-bindgen | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wit-component | 0.254.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wit-parser | 0.254.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| witx | 0.9.1 | Apache-2.0 |
| writeable | 0.6.4 | Unicode-3.0 |
| x11-dl | 2.21.0 | MIT |
| x11rb | 0.13.2 | MIT OR Apache-2.0 |
| x11rb-protocol | 0.13.2 | MIT OR Apache-2.0 |
| xcursor | 0.3.11 | MIT |
| xkbcommon-dl | 0.4.2 | MIT |
| xkeysym | 0.2.1 | MIT OR Apache-2.0 OR Zlib |
| xml-rs | 0.8.29 | MIT |
| yoke | 0.8.3 | Unicode-3.0 |
| yoke-derive | 0.8.2 | Unicode-3.0 |
| zerocopy | 0.8.57 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zerocopy-derive | 0.8.57 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zerofrom | 0.1.8 | Unicode-3.0 |
| zerofrom-derive | 0.1.7 | Unicode-3.0 |
| zerotrie | 0.2.5 | Unicode-3.0 |
| zerovec | 0.11.8 | Unicode-3.0 |
| zerovec-derive | 0.11.6 | Unicode-3.0 |
| zlib-rs | 0.6.7 | Zlib |
| zmij | 1.0.23 | MIT |
| zstd | 0.13.3 | MIT |
| zstd-safe | 7.3.0 | BSD-3-Clause |
| zstd-sys | 2.1.0+zstd.1.5.7 | BSD-3-Clause |
| zune-core | 0.5.3 | MIT OR Apache-2.0 OR Zlib |
| zune-jpeg | 0.5.15 | MIT OR Apache-2.0 OR Zlib |
