# cargo bsize: librolldown_binding.dylib

> Only propose source-code changes. Do not propose configuration changes.

Contents: [Summary](#summary) · [Linker provenance](#linker-provenance) · [Dependencies](#dependencies) · [Functions and data symbols](#functions-and-data-symbols) · [Generic code, by the types it is instantiated over](#generic-code-by-the-types-it-is-instantiated-over) · [Panic, format, and unwind overhead](#panic-format-and-unwind-overhead) · [Duplicate read-only data](#duplicate-read-only-data) · [Dynamic dispatch](#dynamic-dispatch) · [Reference graph](#reference-graph) · [Derives](#derives) · [Largest types](#largest-types) · [Inlined code](#inlined-code) · [Assembly](#assembly) · [Constant data](#constant-data) · [Dynamic relocations](#dynamic-relocations)

## Summary

- Binary: `/Users/boshen/github/rolldown/rolldown/target/bsize/release/librolldown_binding.dylib` (macho)
- Total 22.9 MiB, shipped 15.8 MiB (without symbols and debug info). Shares below are of the shipped size.

|     Size | Share | Category |
|---------:|------:|---|
| 11.6 MiB | 73.5% | code |
|  7.1 MiB |     - | symbols (not shipped) |
|  2.2 MiB | 14.0% | unwind |
|  1.9 MiB | 12.2% | read-only data |
| 24.7 KiB |  0.2% | data |
| 29.6 KiB |  0.2% | overhead (headers, padding, code signature) |

### Sections

|      Size | Share | Section |
|----------:|------:|---|
|  11.5 MiB | 73.0% | `__TEXT,__text` |
|   7.1 MiB |     - | `__LINKEDIT` |
|   1.5 MiB |  9.3% | `__TEXT,__eh_frame` |
|   1.2 MiB |  7.4% | `__TEXT,__const` |
| 646.7 KiB |  4.0% | `__DATA_CONST,__const` |
| 522.1 KiB |  3.2% | `__TEXT,__gcc_except_tab` |
| 243.8 KiB |  1.5% | `__TEXT,__unwind_info` |
| 131.9 KiB |  0.8% | `__TEXT,__cstring` |
|  70.1 KiB |  0.4% | `__TEXT,__text_startup` |
|  21.3 KiB |  0.1% | `__DATA,__data` |

### Coverage

_what the symbol table names, and what it leaves anonymous_

|      Size | Share | Bytes |
|----------:|------:|---|
|  11.6 MiB | 73.5% | code in 24681 named symbols |
| 477.1 KiB |  2.9% | read-only data in 495 named symbols |
|   1.5 MiB |  9.3% | in those sections, named by no symbol |
|  34.0 KiB |  0.2% | code no compile unit claims (built without debug info: C, assembly, std's backtrace crates) |

## Linker provenance

_the final link's input objects, archive members, and exported retention roots; map bytes cover named input regions, not unwind metadata or every output-section byte_

- macho linker map: `/Users/boshen/github/rolldown/rolldown/target/bsize/linker/rolldown_binding.map`; 15.6 MiB across 209 live input objects, including 192 archive members.

### Largest input objects and archive members

|      Size | Share | Input | Regions |
|----------:|------:|---|--------:|
|   3.4 MiB | 21.3% | `target/bsize/release/deps/rolldown_binding.rolldown_binding.2ce10e0935413a03-cgu.0.rcgu.o` |   18731 |
|   2.4 MiB | 15.1% | `target/bsize/release/deps/librolldown-699c213c902e713d.rlib(rolldown-699c213c902e713d.rolldown.9131ee93379a0dd1-cgu.0.rcgu.o)` |   10493 |
| 739.6 KiB |  4.6% | `target/bsize/release/deps/liboxc_transformer-a362365a2a03075d.rlib(oxc_transformer-a362365a2a03075d.oxc_transformer.d8bdb8c6af16394d-cgu.0.rcgu.o)` |    2351 |
| 564.0 KiB |  3.5% | `target/bsize/release/deps/libregex_syntax-1d55d76852f8a6fb.rlib(regex_syntax-1d55d76852f8a6fb.regex_syntax.9ec4c6b8a58bb257-cgu.0.rcgu.o)` |    5637 |
| 515.7 KiB |  3.2% | `target/bsize/release/deps/liboxc_minifier-5b012c07026db1ae.rlib(oxc_minifier-5b012c07026db1ae.oxc_minifier.bf17b2bb8865c816-cgu.0.rcgu.o)` |    1744 |
| 475.8 KiB |  2.9% | `target/bsize/release/deps/libregress-171a9df01bd52927.rlib(regress-171a9df01bd52927.regress.546e0ce0c07c852b-cgu.0.rcgu.o)` |    5007 |
| 396.4 KiB |  2.4% | `target/bsize/release/deps/liboxc_parser-f393195fb277ff1a.rlib(oxc_parser-f393195fb277ff1a.oxc_parser.945d18a4ed5471a0-cgu.0.rcgu.o)` |    1844 |
| 348.9 KiB |  2.2% | `target/bsize/release/deps/librolldown_common-7c933829b947746c.rlib(rolldown_common-7c933829b947746c.rolldown_common.462b64d3358b81b1-cgu.0.rcgu.o)` |    1964 |
| 344.8 KiB |  2.1% | `target/bsize/release/deps/libregex_automata-c3de628cb1176db7.rlib(regex_automata-c3de628cb1176db7.regex_automata.c9d422b0b3782b3a-cgu.0.rcgu.o)` |    2333 |
| 338.5 KiB |  2.1% | `target/bsize/release/deps/liboxc_ast-c21078962098c19b.rlib(oxc_ast-c21078962098c19b.oxc_ast.d2990849e4208841-cgu.0.rcgu.o)` |    1149 |

### Largest static archives

|      Size | Share | Archive | Members | Regions |
|----------:|------:|---|--------:|--------:|
|   2.4 MiB | 15.1% | `target/bsize/release/deps/librolldown-699c213c902e713d.rlib` |       1 |   10493 |
| 739.6 KiB |  4.6% | `target/bsize/release/deps/liboxc_transformer-a362365a2a03075d.rlib` |       1 |    2351 |
| 564.0 KiB |  3.5% | `target/bsize/release/deps/libregex_syntax-1d55d76852f8a6fb.rlib` |       1 |    5637 |
| 515.7 KiB |  3.2% | `target/bsize/release/deps/liboxc_minifier-5b012c07026db1ae.rlib` |       1 |    1744 |
| 475.8 KiB |  2.9% | `target/bsize/release/deps/libregress-171a9df01bd52927.rlib` |       1 |    5007 |
| 396.4 KiB |  2.4% | `target/bsize/release/deps/liboxc_parser-f393195fb277ff1a.rlib` |       1 |    1844 |
| 348.9 KiB |  2.2% | `target/bsize/release/deps/librolldown_common-7c933829b947746c.rlib` |       1 |    1964 |
| 344.8 KiB |  2.1% | `target/bsize/release/deps/libregex_automata-c3de628cb1176db7.rlib` |       1 |    2333 |
| 338.5 KiB |  2.1% | `target/bsize/release/deps/liboxc_ast-c21078962098c19b.rlib` |       1 |    1149 |
| 298.8 KiB |  1.8% | `target/bsize/release/deps/liboxc_resolver-fe2841de7bdbe7a4.rlib` |       1 |    1612 |

### Exported retention roots

_externally visible definitions must survive even when the assembly graph names no caller; size is the root itself, not everything reachable from it_

|     Size | Share | Export |
|---------:|------:|---|
| 10.3 KiB |  0.1% | `_napi_register_module_v1` |
|      4 B | <0.1% | `_blake3_compress_in_place_portable` |

### Largest regions named only by the linker

_local literals and synthesized regions with no final symbol at the same address and no enclosing named code range; these close part of the anonymous-byte gap_

- 3.2 MiB across all such mapped regions.

|     Size | Share | Region | Input |
|---------:|------:|---|---|
| 68.9 KiB |  0.4% | `l_anon.5af0d60ead25e2c9512b8a9d560d5843.3246` | `target/bsize/release/deps/libregex_syntax-1d55d76852f8a6fb.rlib(regex_syntax-1d55d76852f8a6fb.regex_syntax.9ec4c6b8a58bb257-cgu.0.rcgu.o)` |
| 61.8 KiB |  0.4% | `l_anon.552c6a13613e69715a7a18ec8b66cbcf.4915` | `target/bsize/release/deps/libregress-171a9df01bd52927.rlib(regress-171a9df01bd52927.regress.546e0ce0c07c852b-cgu.0.rcgu.o)` |
| 25.2 KiB |  0.2% | `l_anon.552c6a13613e69715a7a18ec8b66cbcf.3277` | `target/bsize/release/deps/libregress-171a9df01bd52927.rlib(regress-171a9df01bd52927.regress.546e0ce0c07c852b-cgu.0.rcgu.o)` |
| 21.9 KiB |  0.1% | `l_anon.552c6a13613e69715a7a18ec8b66cbcf.1471` | `target/bsize/release/deps/libregress-171a9df01bd52927.rlib(regress-171a9df01bd52927.regress.546e0ce0c07c852b-cgu.0.rcgu.o)` |
| 19.0 KiB |  0.1% | `l_anon.096e9e599eb733942966fc66bd280d81.31` | `target/bsize/release/deps/libunicode_segmentation-d19f3cbd4cb0d185.rlib(unicode_segmentation-d19f3cbd4cb0d185.unicode_segmentation.373d03d0ea67f9aa-cgu.0.rcgu.o)` |
| 10.8 KiB |  0.1% | `l_anon.2de71eaba34602eb5455f78829ea9c46.117` | `target/bsize/release/deps/librolldown_plugin_oxc_runtime-fa74ee89a4f3e49d.rlib(rolldown_plugin_oxc_runtime-fa74ee89a4f3e49d.rolldown_plugin_oxc_runtime.6e4441297d2db11b-cgu.0.rcgu.o)` |
| 10.6 KiB |  0.1% | `l_anon.2de71eaba34602eb5455f78829ea9c46.291` | `target/bsize/release/deps/librolldown_plugin_oxc_runtime-fa74ee89a4f3e49d.rlib(rolldown_plugin_oxc_runtime-fa74ee89a4f3e49d.rolldown_plugin_oxc_runtime.6e4441297d2db11b-cgu.0.rcgu.o)` |
| 10.6 KiB |  0.1% | `l_anon.5af0d60ead25e2c9512b8a9d560d5843.4548` | `target/bsize/release/deps/libregex_syntax-1d55d76852f8a6fb.rlib(regex_syntax-1d55d76852f8a6fb.regex_syntax.9ec4c6b8a58bb257-cgu.0.rcgu.o)` |
| 10.4 KiB |  0.1% | `l_anon.552c6a13613e69715a7a18ec8b66cbcf.4255` | `target/bsize/release/deps/libregress-171a9df01bd52927.rlib(regress-171a9df01bd52927.regress.546e0ce0c07c852b-cgu.0.rcgu.o)` |
| 10.0 KiB |  0.1% | `l_anon.75b961e9159ddcb1a928ee3769e65115.696` | `target/bsize/release/deps/liboxc_ecmascript-5066a5ee06b58dda.rlib(oxc_ecmascript-5066a5ee06b58dda.oxc_ecmascript.2addfac04a870905-cgu.0.rcgu.o)` |

## Dependencies

### Duplicate versions (7)

_the same crate at several versions; each ships its own copy of the code, costed here from the compile units when the debug info was read_

| Crate | Version |     Code | Used by |
|---|---|---------:|---|
| `compact_str` | 0.9.1 |    248 B | oxc_resolver 11.24.2 |
|  | 0.10.0 |  3.2 KiB | oxc_span 0.146.0, oxc_str 0.146.0, oxc_transformer 0.146.0 |
| `embedded-io` | 0.4.0 |          | postcard 1.1.3 |
|  | 0.6.1 |          | postcard 1.1.3 |
| `foldhash` | 0.1.5 |          | hashbrown 0.15.5 |
|  | 0.2.0 |  1.4 KiB | hashbrown 0.16.1, hashbrown 0.17.1 |
| `getrandom` | 0.3.4 |          | ahash 0.8.12, tempfile 3.27.0 |
|  | 0.4.3 |    216 B | uuid 1.24.1 |
| `hashbrown` | 0.14.5 |     80 B | dashmap 6.2.1 |
|  | 0.15.5 |          | petgraph 0.8.3 |
|  | 0.16.1 |          | halfbrown 0.4.0, regress 0.11.1 |
|  | 0.17.1 |     80 B | indexmap 2.14.0, oxc_allocator 0.146.0, oxc_str 0.146.0, referencing 0.49.9 |
| `miniz_oxide` | 0.8.9 |          | flate2 1.1.9 |
|  | 0.9.1 |          | oxc-browserslist 5.0.1 |
| `num-bigint` | 0.4.8 |          | num 0.4.3, num-rational 0.4.2 |
|  | 0.5.1 | 58.4 KiB | oxc_ecmascript 0.146.0, oxc_parser 0.146.0, rolldown_common 1.2.5 |

### Features

_each linked dependency's resolved features and who asked for them; the bytes are the crate's whole code, so a shorter feature list returns some part of them_

|      Size | Share | Crate | Features | Requested by |
|----------:|------:|---|---|---|
| 319.0 KiB |  2.0% | `oxc_parser 0.146.0` | default, regular_expression | oxc (default features; regular_expression), oxc_minifier (default features; regular_expression), oxc_minify_napi (default features; regular_expression), oxc_transformer_plugins (default features; regular_expression) |
| 298.4 KiB |  1.8% | `oxc_ast 0.146.0` | default, serialize | oxc (default features), oxc_ast_visit (default features), oxc_codegen (default features), oxc_ecmascript (default features), oxc_isolated_declarations (default features), oxc_mangler (default features), oxc_minifier (default features), oxc_napi (default features), oxc_parser (default features), oxc_semantic (default features), oxc_transformer (default features), oxc_transformer_plugins (default features), oxc_traverse (default features) |
| 271.6 KiB |  1.7% | `regex-automata 0.4.18` | alloc, dfa, dfa-build, dfa-onepass, dfa-search, hybrid, meta, nfa, nfa-backtrack, nfa-pikevm, nfa-thompson, perf, perf-inline, perf-literal, perf-literal-multisubstring, perf-literal-substring, std, syntax, unicode, unicode-age, unicode-bool, unicode-case, unicode-gencat, unicode-perl, unicode-script, unicode-segment, unicode-word-boundary | fancy-regex (alloc, syntax, meta, nfa, dfa, hybrid), globset (std, perf, syntax, meta, nfa, hybrid), ignore (std, perf, syntax, meta, nfa, hybrid, dfa-onepass), regex (alloc, syntax, meta, nfa-pikevm) |
| 240.3 KiB |  1.5% | `oxc_resolver 11.24.2` | default, pnp, yarn_pnp | generator (default features; yarn_pnp), oxc_resolver_napi (default features), rolldown_binding (default features; yarn_pnp), rolldown_common (default features; yarn_pnp), rolldown_error (default features; yarn_pnp), rolldown_fs (default features; yarn_pnp), rolldown_plugin_vite_resolve (default features; yarn_pnp), rolldown_plugin_vite_transform (default features; yarn_pnp), rolldown_resolver (default features; yarn_pnp) |
| 193.0 KiB |  1.2% | `oxc_codegen 0.146.0` | sourcemap | oxc, oxc_minify_napi (sourcemap) |
| 172.5 KiB |  1.1% | `regex-syntax 0.8.11` | default, std, unicode, unicode-age, unicode-bool, unicode-case, unicode-gencat, unicode-perl, unicode-script, unicode-segment | fancy-regex, globset (std), jsonschema-regex (default features), regex, regex-automata |
| 172.4 KiB |  1.1% | `oxc_semantic 0.146.0` | default, serialize | oxc (default features), oxc_codegen (default features), oxc_mangler (default features), oxc_minifier (default features), oxc_transformer (default features), oxc_transformer_plugins (default features), oxc_traverse (default features) |
| 169.3 KiB |  1.0% | `oxc_resolver_napi 11.24.2` | yarn_pnp | rolldown_binding (yarn_pnp) |
| 110.6 KiB |  0.7% | `oxc_transform_napi 0.146.0` | default | rolldown_binding (default features) |
| 107.6 KiB |  0.7% | `regress 0.11.1` | backend-pikevm, default, std | oxc_resolver_napi (default features), pnp (default features), rolldown_utils (default features) |

## Functions and data symbols

### Largest functions

|     Size | Share | Function | Defined at |
|---------:|------:|---|---|
| 44.8 KiB |  0.3% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` | `crates/rolldown/src/module_loader/module_loader.rs:379` |
| 37.1 KiB |  0.2% | `rolldown_binding::utils::normalize_binding_options::normalize_binding_options` | `crates/rolldown_binding/src/utils/normalize_binding_options.rs:460` |
| 36.9 KiB |  0.2% | `<rolldown::stages::link_stage::LinkStage>::link` | `crates/rolldown/src/stages/link_stage/mod.rs:235` |
| 31.2 KiB |  0.2% | `core::ptr::drop_glue::<hashbrown::scopeguard::ScopeGuard<&mut hashbrown::raw::RawTableInner, <hashbrown::raw::RawTableInner>::rehash_in_place::{closure#0}>>` (33×) | `library/core/src/ptr/mod.rs:823` |
| 30.7 KiB |  0.2% | `<regress::classicalbacktrack::MatchAttempter<regress::indexing::Utf8Input>>::run_lookaround::<regress::cursor::Backward>` (4×) | `regress-0.11.1/src/classicalbacktrack.rs:458` |
| 30.5 KiB |  0.2% | `<rolldown::module_finalizers::ScopeHoistingFinalizer as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_program` | `crates/rolldown/src/module_finalizers/impl_visit_mut.rs:180` |
| 29.3 KiB |  0.2% | `rolldown::utils::prepare_build_context::prepare_build_context` | `crates/rolldown/src/utils/prepare_build_context.rs:163` |
| 28.9 KiB |  0.2% | `<regress::parse::Parser<core::iter::adapters::map::Map<core::str::iter::Chars, <u32 as core::convert::From<char>>::from>>>::consume_term` (3×) | `regress-0.11.1/src/parse.rs:513` |
| 28.2 KiB |  0.2% | `<regress::classicalbacktrack::MatchAttempter<regress::indexing::Utf8Input>>::try_at_pos::<regress::cursor::Forward>` (4×) | `regress-0.11.1/src/classicalbacktrack.rs:637` |
| 27.1 KiB |  0.2% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` | `crates/rolldown/src/ecmascript/ecma_generator.rs:48` |

### Largest data symbols

_≤ marks an upper bound: the size runs to the next symbol, so it also counts the unnamed constants in between_

|        Size | Share | Symbol |
|------------:|------:|---|
| ≤ 296.0 KiB |  1.8% | `__mi_page_empty` |
|    17.2 KiB |  0.1% | `zmij::STATIC_DATA` |
|    12.7 KiB |  0.1% | `unicode_linebreak::BREAK_PROP_TRIE_DATA` |
|  ≤ 11.6 KiB |  0.1% | `__mi_theap_empty_wrong` |
|  ≤ 10.2 KiB |  0.1% | `core::num::imp::dec2flt::table::POWER_OF_FIVE_128` |
|     9.7 KiB |  0.1% | `dragonbox_ecma::cache::CACHE` |
|     8.0 KiB | <0.1% | `zlib_rs::crc32::braid::CRC32_WORD_TABLE` |
|   ≤ 7.9 KiB | <0.1% | `__mi_theap_empty` |
|     7.8 KiB | <0.1% | `unicode_id_start::tables::LEAF` |
|     5.8 KiB | <0.1% | `unicode_width::tables::WIDTH_LEAVES` |

### By pattern

_a symbol can match several patterns, so these do not sum to the total; the ↳ rows are the largest single offenders_

|      Size | Share | Pattern | Symbols |
|----------:|------:|---|--------:|
|   2.6 MiB | 16.3% | closures |    4084 |
| 227.6 KiB |  1.4% | ↳ `core::ptr::drop_glue` |         |
| 115.2 KiB |  0.7% | ↳ `core::slice::sort::stable::quicksort::quicksort` |         |
| 103.1 KiB |  0.6% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |         |
|   1.1 MiB |  7.1% | drop glue |    5774 |
|   1.1 MiB |  7.1% | ↳ `core::ptr::drop_glue` |         |
|   1.5 KiB | <0.1% | ↳ `core::ptr::drop_in_place` |         |
| 532.9 KiB |  3.3% | iterators |     961 |
|  45.4 KiB |  0.3% | ↳ `rayon::iter::plumbing::bridge_producer_consumer::helper` |         |
|  44.5 KiB |  0.3% | ↳ `core::ptr::drop_glue` |         |
|  37.8 KiB |  0.2% | ↳ `rayon_core::join::join_context::{closure#0}` |         |
| 401.3 KiB |  2.5% | formatting |    2836 |
|  11.2 KiB |  0.1% | ↳ `<std::thread::local::LocalKey<core::cell::RefCell<alloc::string::String>>>::with` |         |
|  10.9 KiB |  0.1% | ↳ `<alloc::string::String as core::fmt::Write>::write_char` |         |
|  10.4 KiB |  0.1% | ↳ `<&regress::insn::Insn as core::fmt::Debug>::fmt` |         |
| 122.9 KiB |  0.8% | serde |     180 |
|  12.8 KiB |  0.1% | ↳ `<&mut serde_json::de::Deserializer<serde_json::read::SliceRead> as serde_core::de::Deserializer>::deserialize_struct` |         |
|  11.9 KiB |  0.1% | ↳ `<serde_json::value::Value as serde_core::de::Deserialize>::deserialize` |         |
|  10.5 KiB |  0.1% | ↳ `pnp::manifest::deserialize_package_registry_data` |         |
|  31.1 KiB |  0.2% | panic paths |     131 |
|  10.4 KiB |  0.1% | ↳ `core::ptr::drop_glue` |         |
|   7.0 KiB | <0.1% | ↳ `<regex_automata::util::pool::inner::Pool<regex_automata::meta::regex::Cache, alloc::boxed::Box<dyn core::ops::function::Fn<(), Output = regex_automata::meta::regex::Cache> + core::marker::Sync + core::marker::Send + core::panic::unwind_safe::UnwindSafe + core::panic::unwind_safe::RefUnwindSafe>>>::put_value` |         |
|   5.0 KiB | <0.1% | ↳ `<regex_automata::util::pool::inner::Pool<regex_automata::meta::regex::Cache, alloc::boxed::Box<dyn core::ops::function::Fn<(), Output = regex_automata::meta::regex::Cache> + core::marker::Sync + core::marker::Send + core::panic::unwind_safe::UnwindSafe + core::panic::unwind_safe::RefUnwindSafe>>>::get_slow` |         |

### By trait method, every impl combined

_attribution — one method summed over every impl, so where the bytes sit, not what one change removes_

|      Size | Share | Trait method | Impls |
|----------:|------:|---|------:|
| 254.1 KiB |  1.6% | `<core::fmt::Debug>::fmt` |  1986 |
| 245.2 KiB |  1.5% | `<napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |   171 |
| 143.3 KiB |  0.9% | `<oxc_estree::serialize::ESTree>::serialize` |    68 |
| 118.2 KiB |  0.7% | `<oxc_codegen::gen::Gen>::gen` |   124 |
| 100.2 KiB |  0.6% | `<napi::bindgen_runtime::js_values::ToNapiValue>::to_napi_value` |   133 |
|  75.9 KiB |  0.5% | `<core::iter::traits::iterator::Iterator>::next` |    88 |
|  72.7 KiB |  0.4% | `<core::convert::From>::from` |   142 |
|  68.2 KiB |  0.4% | `<oxc_allocator::clone_in::CloneIn>::clone_in_impl` |   103 |
|  62.6 KiB |  0.4% | `<core::clone::Clone>::clone` |    84 |
|  51.5 KiB |  0.3% | `<core::fmt::Display>::fmt` |   399 |

### By trait, every method of every impl combined

_one axis coarser than above; the ↳ rows are the trait's largest single impls — the concrete targets_

|      Size | Share | Trait | Methods |
|----------:|------:|---|--------:|
| 254.1 KiB |  1.6% | `core::fmt::Debug` |    1986 |
|  10.4 KiB |  0.1% | ↳ `<&regress::insn::Insn as core::fmt::Debug>::fmt` |         |
|   7.4 KiB | <0.1% | ↳ `<oxc_resolver::error::ResolveError as core::fmt::Debug>::fmt` |         |
|   2.6 KiB | <0.1% | ↳ `<&[u8; 16] as core::fmt::Debug>::fmt` |         |
| 245.2 KiB |  1.5% | `napi::bindgen_runtime::js_values::FromNapiValue` |     171 |
|  11.9 KiB |  0.1% | ↳ `<rolldown_binding::options::binding_output_options::BindingOutputOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |         |
|  10.1 KiB |  0.1% | ↳ `<rolldown_binding::options::binding_input_options::BindingInputOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |         |
|   9.0 KiB |  0.1% | ↳ `<rolldown_binding::options::plugin::binding_plugin_options::BindingPluginOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |         |
| 186.9 KiB |  1.2% | `oxc_ast_visit::generated::visit_js_mut::VisitJsMut` |     280 |
|  30.5 KiB |  0.2% | ↳ `<rolldown::module_finalizers::ScopeHoistingFinalizer as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_program` |         |
|   7.7 KiB | <0.1% | ↳ `<rolldown::module_finalizers::ScopeHoistingFinalizer as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_expression` |         |
|   5.1 KiB | <0.1% | ↳ `<rolldown::utils::tweak_ast_for_scanning::PreProcessor as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_expression` |         |
| 177.7 KiB |  1.1% | `rolldown_plugin::plugin::Plugin` |     139 |
|  10.5 KiB |  0.1% | ↳ `<rolldown_plugin_vite_resolve::vite_resolve_plugin::ViteResolvePlugin as rolldown_plugin::plugin::Plugin>::resolve_id::{closure#0}` |         |
|   8.1 KiB | <0.1% | ↳ `<rolldown_plugin_vite_manifest::ViteManifestPlugin as rolldown_plugin::plugin::Plugin>::generate_bundle::{closure#0}` |         |
|   6.3 KiB | <0.1% | ↳ `<rolldown_binding::options::plugin::js_plugin::JsPlugin as rolldown_plugin::plugin::Plugin>::load::{closure#0}` |         |
| 143.3 KiB |  0.9% | `oxc_estree::serialize::ESTree` |      68 |
|  21.4 KiB |  0.1% | ↳ `<oxc_ast::ast::js::Expression as oxc_estree::serialize::ESTree>::serialize` |         |
|  16.7 KiB |  0.1% | ↳ `<oxc_ast::ast::js::Statement as oxc_estree::serialize::ESTree>::serialize` |         |
|  13.2 KiB |  0.1% | ↳ `<oxc_ast::ast::ts::TSType as oxc_estree::serialize::ESTree>::serialize` |         |
| 141.3 KiB |  0.9% | `oxc_ast_visit::generated::visit_js::VisitJs` |     227 |
|  12.0 KiB |  0.1% | ↳ `<rolldown::ast_scanner::AstScanner as oxc_ast_visit::generated::visit_js::VisitJs>::visit_statement` |         |
|   7.2 KiB | <0.1% | ↳ `<rolldown::ast_scanner::AstScanner as oxc_ast_visit::generated::visit_js::VisitJs>::visit_identifier_reference` |         |
|   3.9 KiB | <0.1% | ↳ `<oxc_minifier::keep_var::KeepVar as oxc_ast_visit::generated::visit_js::VisitJs>::visit_expression` |         |
| 119.7 KiB |  0.7% | `oxc_codegen::gen::Gen` |     129 |
|   7.5 KiB | <0.1% | ↳ `<oxc_ast::ast::js::Statement as oxc_codegen::gen::Gen>::gen` |         |
|   3.9 KiB | <0.1% | ↳ `<oxc_ast::ast::js::ImportDeclaration as oxc_codegen::gen::Gen>::gen` |         |
|   3.2 KiB | <0.1% | ↳ `<oxc_ast::ast::js::IfStatement as oxc_codegen::gen::Gen>::gen` |         |
| 118.9 KiB |  0.7% | `oxc_ast_visit::generated::visit::Visit` |     195 |
|   3.7 KiB | <0.1% | ↳ `<oxc_semantic::stats::Counter as oxc_ast_visit::generated::visit::Visit>::visit_expression` |         |
|   3.1 KiB | <0.1% | ↳ `<oxc_minifier::property_mangler::PropertyCollector as oxc_ast_visit::generated::visit::Visit>::visit_expression` |         |
|   2.9 KiB | <0.1% | ↳ `<oxc_semantic::builder::SemanticBuilder as oxc_ast_visit::generated::visit::Visit>::visit_ts_enum_declaration` |         |
| 110.7 KiB |  0.7% | `oxc_traverse::generated::traverse::Traverse` |      78 |
|  10.5 KiB |  0.1% | ↳ `<oxc_transformer::es2018::async_generator_functions::AsyncGeneratorFunctions as oxc_traverse::generated::traverse::Traverse<oxc_transformer::state::TransformState>>::enter_statement` |         |
|   9.1 KiB |  0.1% | ↳ `<rolldown::hmr::hmr_ast_finalizer::HmrAstFinalizer as oxc_traverse::generated::traverse::Traverse<()>>::enter_program` |         |
|   8.6 KiB |  0.1% | ↳ `<oxc_transformer::typescript::TypeScript as oxc_traverse::generated::traverse::Traverse<oxc_transformer::state::TransformState>>::enter_class` |         |
| 107.0 KiB |  0.7% | `core::iter::traits::iterator::Iterator` |     149 |
|  12.2 KiB |  0.1% | ↳ `<core::slice::iter::IterMut<rolldown_common::chunk::Chunk> as core::iter::traits::iterator::Iterator>::for_each` |         |
|   6.9 KiB | <0.1% | ↳ `<oxc_diagnostics::handlers::graphical::line::CharWidthIterator as core::iter::traits::iterator::Iterator>::next` |         |
|   3.7 KiB | <0.1% | ↳ `<core::iter::adapters::filter_map::FilterMap<indexmap::map::iter::Iter<rolldown_utils::bitset::BitSet, rolldown_common::types::chunk_idx::ChunkIdx>, <rolldown::stages::generate_stage::GenerateStage>::try_insert_common_module_to_exist_chunk::{closure#1}> as core::iter::traits::iterator::Iterator>::next` |         |

### By crate, where the code is defined

|      Size | Share | Crate | Symbols |
|----------:|------:|---|--------:|
|   2.0 MiB | 12.8% | `core` |    8504 |
|   1.2 MiB |  7.7% | `rolldown` |     556 |
| 806.1 KiB |  5.0% | `rolldown_binding` |     813 |
| 475.1 KiB |  2.9% | `napi` |     656 |
| 414.7 KiB |  2.6% | `hashbrown` |     936 |
| 400.2 KiB |  2.5% | `oxc_transformer` |     476 |
| 398.4 KiB |  2.5% | `oxc_ast_visit` |     498 |
| 295.4 KiB |  1.8% | `oxc_minifier` |     366 |
| 285.7 KiB |  1.8% | `oxc_ast` |     504 |
| 278.8 KiB |  1.7% | `regress` |     177 |

### By workspace file, where the code is defined

_2.8 MiB (17.9%) in 3006 functions defined in this workspace; code inlined away is charged to its caller's file, and a generated file shows the full cost of what it generates_

|     Size | Share | File | Functions |
|---------:|------:|---|----------:|
| 85.3 KiB |  0.5% | `crates/rolldown_binding/src/utils/normalize_binding_options.rs` |        47 |
| 73.4 KiB |  0.5% | `crates/rolldown/src/module_loader/module_loader.rs` |        10 |
| 68.2 KiB |  0.4% | `crates/rolldown_binding/src/types/binding_magic_string.rs` |        99 |
| 64.5 KiB |  0.4% | `crates/rolldown_binding/src/options/plugin/js_plugin.rs` |        23 |
| 57.4 KiB |  0.4% | `crates/rolldown/src/stages/link_stage/bind_imports_and_exports.rs` |        10 |
| 53.5 KiB |  0.3% | `crates/rolldown/src/stages/generate_stage/compute_cross_chunk_links.rs` |         9 |
| 53.0 KiB |  0.3% | `crates/rolldown/src/stages/generate_stage/mod.rs` |         9 |
| 49.8 KiB |  0.3% | `crates/rolldown_plugin/src/pluginable.rs` |       732 |
| 46.7 KiB |  0.3% | `crates/rolldown/src/module_finalizers/impl_visit_mut.rs` |         9 |
| 46.5 KiB |  0.3% | `crates/rolldown/src/stages/generate_stage/order_wrapping.rs` |        16 |

### By workspace directory

|      Size | Share | Directory | Functions |
|----------:|------:|---|----------:|
| 372.7 KiB |  2.3% | `crates/rolldown/src/stages/generate_stage` |       122 |
| 176.3 KiB |  1.1% | `crates/rolldown_binding/src/types` |       275 |
| 154.3 KiB |  1.0% | `crates/rolldown_binding/src/options/plugin` |        99 |
| 124.4 KiB |  0.8% | `crates/rolldown/src/stages/link_stage` |        25 |
| 122.5 KiB |  0.8% | `crates/rolldown/src/module_loader` |        21 |
| 122.4 KiB |  0.8% | `crates/rolldown_binding/src` |       130 |
|  98.7 KiB |  0.6% | `crates/rolldown_binding/src/utils` |        62 |
|  86.9 KiB |  0.5% | `crates/rolldown/src/utils` |        33 |
|  78.6 KiB |  0.5% | `crates/rolldown/src/module_finalizers` |        38 |
|  74.3 KiB |  0.5% | `crates/rolldown_plugin/src/plugin_driver` |        44 |

### Generic families

_every instantiation of one generic summed; recoverable = the total less its largest instance, what collapsing the family onto one copy would return_

|      Size | Share | Family | Instances |    Each | Recoverable | Defined at |
|----------:|------:|---|----------:|--------:|------------:|---|
|   1.1 MiB |  7.1% | `core::ptr::drop_glue` |      5755 |   203 B |    ~1.1 MiB | `library/core/src/ptr/mod.rs:823` |
| 136.4 KiB |  0.8% | `core::slice::sort::unstable::quicksort::quicksort` |        53 | 2.6 KiB |  ~126.6 KiB | `library/core/src/slice/sort/unstable/quicksort.rs:21` |
| 128.0 KiB |  0.8% | `core::slice::sort::stable::quicksort::quicksort` |        39 | 3.3 KiB |  ~119.3 KiB | `library/core/src/slice/sort/stable/quicksort.rs:16` |
|  81.5 KiB |  0.5% | `napi::bindgen_runtime::js_values::object::from_raw_optional_field` |       129 |   646 B |   ~78.3 KiB | `napi-3.12.2/src/bindgen_runtime/js_values/object.rs:774` |
|  75.0 KiB |  0.5% | `napi::threadsafe_function::call_js_cb` |        43 | 1.7 KiB |   ~67.8 KiB | `napi-3.12.2/src/threadsafe_function.rs:805` |
|  64.2 KiB |  0.4% | `core::slice::sort::stable::drift::sort` |        39 | 1.6 KiB |   ~61.1 KiB | `library/core/src/slice/sort/stable/drift.rs:20` |
|  45.4 KiB |  0.3% | `rayon::iter::plumbing::bridge_producer_consumer::helper` |        39 | 1.2 KiB |   ~41.2 KiB | `rayon-1.12.0/src/iter/plumbing/mod.rs:393` |
|  45.4 KiB |  0.3% | `<hashbrown::raw::RawTable<usize>>::reserve_rehash` |        39 | 1.2 KiB |   ~43.9 KiB | `hashbrown-0.17.1/src/raw.rs:948` |
|  41.2 KiB |  0.3% | `tokio::runtime::task::raw::poll` |        63 |   668 B |   ~40.0 KiB | `tokio-1.53.1/src/runtime/task/raw.rs:341` |
|  37.8 KiB |  0.2% | `rayon_core::join::join_context::{closure#0}` |        41 |   943 B |   ~36.5 KiB | `rayon-core-1.13.0/src/join/mod.rs:132` |

### By crate, which one caused the instantiation

_generic code from the families above, re-attributed to the crate that instantiated it — not additional_

|      Size | Share | Crate | Symbols |
|----------:|------:|---|--------:|
|   1.3 MiB |  8.4% | `rolldown_binding` |    3324 |
| 524.1 KiB |  3.2% | `rolldown` |    2048 |
| 105.6 KiB |  0.7% | `rolldown_utils` |     224 |
| 104.2 KiB |  0.6% | `oxc_resolver_napi` |     229 |
| 103.1 KiB |  0.6% | `pnp` |     189 |
|  81.9 KiB |  0.5% | `std` |     143 |
|  77.2 KiB |  0.5% | `rolldown_watcher` |     301 |
|  70.1 KiB |  0.4% | `rolldown_common` |     306 |
|  66.4 KiB |  0.4% | `rolldown_dev` |     277 |
|  56.4 KiB |  0.3% | `oxc_resolver` |     209 |

## Generic code, by the types it is instantiated over

_a turbofish names the types a generic was specialized to; bytes count toward every crate those types name, so rows overlap — the ↳ rows are the largest generic families within each_

- 6.4 MiB (40.4%) in 10693 symbols and 330668 inlined instances

|      Size | Share | Argument crate | Instantiations |
|----------:|------:|---|---------------:|
| 975.5 KiB |  6.0% | `rolldown` |          10787 |
| 144.5 KiB |  0.9% | ↳ `core::ptr::drop_glue` |                |
|  67.1 KiB |  0.4% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |                |
|  60.5 KiB |  0.4% | ↳ `core::slice::sort::stable::quicksort::quicksort` |                |
| 879.3 KiB |  5.4% | `rolldown_common` |          11851 |
| 315.8 KiB |  2.0% | ↳ `core::ptr::drop_glue` |                |
|  81.5 KiB |  0.5% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |                |
|  34.8 KiB |  0.2% | ↳ `rayon::iter::plumbing::bridge_producer_consumer::helper` |                |
| 487.1 KiB |  3.0% | `rolldown_binding` |           4773 |
| 153.3 KiB |  0.9% | ↳ `core::ptr::drop_glue` |                |
|  75.0 KiB |  0.5% | ↳ `napi::threadsafe_function::call_js_cb` |                |
|  40.9 KiB |  0.3% | ↳ `napi::bindgen_runtime::js_values::object::from_raw_optional_field` |                |
| 440.0 KiB |  2.7% | `napi` |           3954 |
| 137.8 KiB |  0.9% | ↳ `core::ptr::drop_glue` |                |
|  75.0 KiB |  0.5% | ↳ `napi::threadsafe_function::call_js_cb` |                |
|  56.8 KiB |  0.4% | ↳ `napi::bindgen_runtime::js_values::object::from_raw_optional_field` |                |
| 437.6 KiB |  2.7% | `rustc_hash` |           5780 |
| 131.7 KiB |  0.8% | ↳ `core::ptr::drop_glue` |                |
|   8.9 KiB |  0.1% | ↳ `rayon_core::join::join_context::{closure#0}` |                |
|   7.3 KiB | <0.1% | ↳ `napi::bindgen_runtime::js_values::object::from_raw_optional_field` |                |
| 411.4 KiB |  2.5% | `(primitives)` |           1274 |
| 110.8 KiB |  0.7% | ↳ `core::ptr::copy_nonoverlapping` |                |
|  36.8 KiB |  0.2% | ↳ `core::sync::atomic::atomic_sub` |                |
|  35.8 KiB |  0.2% | ↳ `core::sync::atomic::atomic_load` |                |
| 354.7 KiB |  2.2% | `alloc` |           2702 |
| 177.5 KiB |  1.1% | ↳ `core::ptr::drop_glue` |                |
|  11.8 KiB |  0.1% | ↳ `napi::bindgen_runtime::js_values::object::from_raw_optional_field` |                |
|  11.8 KiB |  0.1% | ↳ `<oxc_diagnostics::handlers::graphical::handler::GraphicalReportHandler>::render_context` |                |
| 333.9 KiB |  2.1% | `core` |           2255 |
|  85.3 KiB |  0.5% | ↳ `core::ptr::drop_glue` |                |
|  49.9 KiB |  0.3% | ↳ `core::ptr::read` |                |
|  36.0 KiB |  0.2% | ↳ `core::str::validations::next_code_point` |                |
| 311.2 KiB |  1.9% | `oxc_transformer` |           2944 |
|  12.5 KiB |  0.1% | ↳ `core::ptr::drop_glue` |                |
|  11.2 KiB |  0.1% | ↳ `oxc_ast_visit::generated::visit_js_mut::walk_js_mut::walk_expression` |                |
|   9.6 KiB |  0.1% | ↳ `oxc_ast_visit::generated::visit_mut::walk_mut::walk_ts_type` |                |
| 303.2 KiB |  1.9% | `hashbrown` |           3824 |
|  78.2 KiB |  0.5% | ↳ `core::ptr::drop_glue` |                |
|   5.1 KiB | <0.1% | ↳ `<hashbrown::raw::RawTable<(alloc::string::String, ())>>::reserve_rehash` |                |
|   2.1 KiB | <0.1% | ↳ `core::ptr::copy` |                |

## Panic, format, and unwind overhead

_infrastructure the code views only hint at; panic="abort" drops the unwind tables, -Zbuild-std with panic_immediate_abort strips the panic locations, disabling tracing removes the callsite metadata_

|     Size | Share | What | Symbols |
|---------:|------:|---|--------:|
|  2.2 MiB | 14.0% | unwind and exception tables |         |
| 31.0 KiB |  0.2% | tracing metadata |     268 |
|  1.5 KiB | <0.1% | panic |       1 |

### Largest unwind metadata owners

_exact per-function frame records plus cleanup-table spans inferred from adjacent LSDA addresses; 2.0 MiB (99.7%) of 2.0 MiB in those two tables joins to 16167 named functions, while shared CIEs, compact-unwind indexes, padding, and unnamed code remain only in the total above: 19809 of 19809 FDEs named, 7397 functions carry cleanup data_

|    Size | Share | Function |   Frame | Cleanup |     Code | FDEs | LSDAs | Defined at |
|--------:|------:|---|--------:|--------:|---------:|-----:|------:|---|
| 5.6 KiB | <0.1% | `<alloc::raw_vec::RawVecInner>::finish_grow` | 5.6 KiB |     0 B | 15.8 KiB |   94 |     0 | `library/alloc/src/raw_vec/mod.rs:544` |
| 4.4 KiB | <0.1% | `<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>` | 4.4 KiB |     0 B | 10.2 KiB |   79 |     0 | `library/alloc/src/raw_vec/mod.rs:662` |
| 3.7 KiB | <0.1% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |   104 B | 3.6 KiB | 44.8 KiB |    1 |     1 | `crates/rolldown/src/module_loader/module_loader.rs:379` |
| 3.6 KiB | <0.1% | `<std::sys::sync::once_box::OnceBox<std::sys::pal::unix::sync::mutex::Mutex>>::initialize::<<std::sys::sync::mutex::pthread::Mutex>::get::{closure#0}>` | 2.1 KiB | 1.5 KiB |  7.6 KiB |   30 |    30 | `library/std/src/sys/sync/once_box.rs:62` |
| 3.5 KiB | <0.1% | `<hashbrown::raw::RawTableInner>::rehash_in_place` | 2.4 KiB | 1.1 KiB | 17.2 KiB |   28 |    28 | `hashbrown-0.17.1/src/raw.rs:2984` |
| 2.5 KiB | <0.1% | `<alloc::string::String as core::fmt::Write>::write_char` | 2.5 KiB |     0 B | 10.9 KiB |   40 |     0 | `library/alloc/src/string.rs:3377` |
| 2.5 KiB | <0.1% | `<alloc::string::String as core::fmt::Write>::write_str` | 2.5 KiB |     0 B |  5.1 KiB |   40 |     0 | `library/alloc/src/string.rs:3371` |
| 2.5 KiB | <0.1% | `rolldown_binding::utils::normalize_binding_options::normalize_binding_options` |   104 B | 2.4 KiB | 37.1 KiB |    1 |     1 | `crates/rolldown_binding/src/utils/normalize_binding_options.rs:460` |
| 2.4 KiB | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |   104 B | 2.3 KiB | 27.1 KiB |    1 |     1 | `crates/rolldown/src/ecmascript/ecma_generator.rs:48` |
| 2.3 KiB | <0.1% | `core::ptr::drop_glue::<hashbrown::scopeguard::ScopeGuard<&mut hashbrown::raw::RawTableInner, <hashbrown::raw::RawTableInner>::rehash_in_place::{closure#0}>>` | 2.3 KiB |     0 B | 31.2 KiB |   33 |     0 | `library/core/src/ptr/mod.rs:823` |

### Unwind metadata by crate

|      Size | Share | Crate |     Frame |   Cleanup | Functions | With cleanup |
|----------:|------:|---|----------:|----------:|----------:|-------------:|
| 622.0 KiB |  3.8% | `core` | 512.4 KiB | 109.6 KiB |      4586 |         2236 |
| 135.7 KiB |  0.8% | `rolldown_binding` |  65.0 KiB |  70.7 KiB |       810 |          757 |
| 103.8 KiB |  0.6% | `rolldown` |  45.1 KiB |  58.7 KiB |       531 |          264 |
|  86.0 KiB |  0.5% | `napi` |  49.8 KiB |  36.1 KiB |       551 |          430 |
|  78.2 KiB |  0.5% | `alloc` |  69.2 KiB |   9.0 KiB |       647 |          292 |
|  58.0 KiB |  0.4% | `tokio` |  32.8 KiB |  25.2 KiB |       442 |          375 |
|  56.1 KiB |  0.3% | `std` |  39.0 KiB |  17.1 KiB |       416 |          203 |
|  54.3 KiB |  0.3% | `hashbrown` |  48.9 KiB |   5.4 KiB |       483 |          112 |
|  51.6 KiB |  0.3% | `rolldown_plugin` |  38.6 KiB |  13.0 KiB |       672 |          125 |
|  46.9 KiB |  0.3% | `oxc_transformer` |  40.9 KiB |   6.0 KiB |       474 |          108 |

## Duplicate read-only data

_byte-identical constants under different names; the linker's --icf or sharing one const collapses them — 480 B recoverable from 4 groups (8 symbols)_

| Recoverable | Share | Constants | Copies |  Each |
|------------:|------:|---|-------:|------:|
|       256 B | <0.1% | `oxc_estree::serialize::strings::ESCAPE` ≡ `serde_json::ser::ESCAPE` |      2 | 256 B |
|       200 B | <0.1% | `dragonbox_ecma::to_chars::RADIX_100_TABLE` ≡ `itoa::DECIMAL_PAIRS` |      2 | 200 B |
|        16 B | <0.1% | `oxc_estree::serialize::strings::write_char_escape::HEX_DIGITS` ≡ `serde_json::ser::Formatter::write_char_escape::HEX_DIGITS` |      2 |  16 B |
|         8 B | <0.1% | `crossbeam_epoch::guard::unprotected::UNPROTECTED` ≡ `regex_automata::util::pool::inner::THREAD_ID_UNOWNED` |      2 |   8 B |

## Dynamic dispatch

### Named vtables and shims

_a proxy: the few vtables and fn-pointer shims that carry a symbol; most vtables are anonymous and counted below_

- 17.5 KiB (0.1%) in 121 vtables; 0 B (0.0%) in 0 coercion and drop shims.

|  Size | Share | Symbol |
|------:|------:|---|
| 572 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_plugin_context::BindingPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, rolldown_binding::types::binding_rendered_chunk::BindingRenderedChunk, rolldown_binding::types::binding_normalized_options::BindingNormalizedOptions, rolldown_binding::options::plugin::types::binding_render_chunk_meta_chunks::BindingRenderedChunkMeta)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_render_chunk_output::BindingHookRenderChunkOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_render_chunk_output::BindingHookRenderChunkOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_plugin_context::BindingPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, rolldown_binding::types::binding_rendered_chunk::BindingRenderedChunk, rolldown_binding::types::binding_normalized_options::BindingNormalizedOptions, rolldown_binding::options::plugin::types::binding_render_chunk_meta_chunks::BindingRenderedChunkMeta)>, napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_render_chunk_output::BindingHookRenderChunkOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_render_chunk_output::BindingHookRenderChunkOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 512 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_load_context::BindingLoadPluginContext, alloc::string::String)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_load_context::BindingLoadPluginContext, alloc::string::String)>, napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 512 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_transform_context::BindingTransformPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, alloc::string::String, rolldown_binding::options::plugin::types::binding_plugin_transform_extra_args::BindingTransformHookExtraArgs)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_transform_context::BindingTransformPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, alloc::string::String, rolldown_binding::options::plugin::types::binding_plugin_transform_extra_args::BindingTransformHookExtraArgs)>, napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 464 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<(), napi::bindgen_runtime::js_values::either::Either<alloc::vec::Vec<rolldown_binding::types::defer_sync_scan_data::BindingDeferSyncScanData>, rolldown_binding::types::js_callback::InvalidReturnValue>, (), napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<alloc::vec::Vec<rolldown_binding::types::defer_sync_scan_data::BindingDeferSyncScanData>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 432 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<<notify::poll::PollWatcher>::run::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 416 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 416 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<<rolldown::stages::scan_stage::ScanStage<rolldown_fs::os::OsFileSystem>>::create_sourcemap_channel::{closure#0}, std::collections::hash::map::HashMap<rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::vec::Vec<rolldown_common::types::sourcemap_chain_element::SourcemapChainElement>, rustc_hash::FxBuildHasher>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 412 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<notify_debouncer_full::new_debouncer_opt<rolldown_fs_watcher::utils::DebounceEventHandlerAdapter<rolldown_dev::watcher_event_handler::WatcherEventHandler>, notify::poll::PollWatcher, notify_debouncer_full::file_id_map::FileIdMap>::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 396 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<notify_debouncer_full::new_debouncer_opt<rolldown_fs_watcher::utils::DebounceEventHandlerAdapter<rolldown_watcher::task_fs_event_handler::TaskFsEventHandler>, notify::poll::PollWatcher, notify_debouncer_full::file_id_map::FileIdMap>::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 372 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<(), napi::bindgen_runtime::js_values::either::Either<std::collections::hash::map::HashMap<alloc::string::String, alloc::string::String, rustc_hash::FxBuildHasher>, rolldown_binding::types::js_callback::InvalidReturnValue>, (), napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<std::collections::hash::map::HashMap<alloc::string::String, alloc::string::String, rustc_hash::FxBuildHasher>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |

### Vtables by trait object

_recovered from the function pointers each anonymous vtable carries; bytes are the vtables themselves, not the methods they point at_

|     Size | Share | Trait object | Vtables |
|---------:|------:|---|--------:|
| 14.4 KiB |  0.1% | `dyn rolldown_plugin::plugin::Plugin` |     461 |
|  5.8 KiB | <0.1% | `dyn core::any::Any` |      16 |
|  2.3 KiB | <0.1% | `dyn core::fmt::Debug` |      50 |
|  2.2 KiB | <0.1% | `dyn core::ops::function::FnOnce` |      62 |
|  2.1 KiB | <0.1% | `dyn core::error::Error` |      24 |
|  1.9 KiB | <0.1% | `dyn core::fmt::Display` |      49 |
|  1.2 KiB | <0.1% | `dyn (trait not named by any slot)` |      39 |
|    768 B | <0.1% | `dyn core::convert::From` |      19 |
|    288 B | <0.1% | `dyn rolldown_error::build_diagnostic::events::AsAnyMut` |       4 |
|    144 B | <0.1% | `dyn core::fmt::Write` |       3 |

## Reference graph

_who calls, addresses, and points at whom, read from the assembly; conservative, since an indirect call it cannot name may retain more_

### Called from one place

_nothing else reaches these — each exists for a single call site, where merging or inlining it would land_

|     Size | Share | Function | Only caller | Defined at |
|---------:|------:|---|---|---|
| 44.8 KiB |  0.3% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}` | `crates/rolldown/src/module_loader/module_loader.rs:379` |
| 36.9 KiB |  0.2% | `<rolldown::stages::link_stage::LinkStage>::link` | `<rolldown::bundle::bundle::Bundle>::bundle_up::{closure#0}` | `crates/rolldown/src/stages/link_stage/mod.rs:235` |
| 27.1 KiB |  0.2% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` | `<rolldown::stages::generate_stage::GenerateStage>::instantiate_chunks::{closure#0}::{closure#0}::{closure#2}::{closure#0}` | `crates/rolldown/src/ecmascript/ecma_generator.rs:48` |
| 26.3 KiB |  0.2% | `<rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}` | `<rolldown::bundler::bundler::Bundler>::compute_hmr_update_for_file_changes::{closure#0}::{closure#0}` | `crates/rolldown/src/hmr/hmr_stage.rs:103` |
| 25.2 KiB |  0.2% | `<rolldown::stages::generate_stage::GenerateStage>::optimize_dynamic_entry_bits` | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/generate_stage/dynamic_already_loaded.rs:56` |
| 20.9 KiB |  0.1% | `rolldown::utils::chunk::deconflict_chunk_symbols::deconflict_chunk_symbols` | `rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::iter::enumerate::EnumerateProducer<rayon::slice::IterMutProducer<rolldown_common::chunk::Chunk>>, rayon::iter::map::MapConsumer<rayon::iter::for_each::ForEachConsumer<<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}::{closure#4}::{closure#0}>, <oxc_index::IndexVec<rolldown_common::types::chunk_idx::ChunkIdx, rolldown_common::chunk::Chunk> as rolldown_utils::index_vec_ext::none_wasm::IndexVecExt<rolldown_common::types::chunk_idx::ChunkIdx, rolldown_common::chunk::Chunk>>::par_iter_mut_enumerated::{closure#0}>>` | `crates/rolldown/src/utils/chunk/deconflict_chunk_symbols.rs:19` |
| 20.7 KiB |  0.1% | `rolldown_plugin_bundle_analyzer::render_markdown::render_markdown` | `<rolldown_plugin_bundle_analyzer::BundleAnalyzerPlugin as rolldown_plugin::plugin::Plugin>::generate_bundle::{closure#0}` | `crates/rolldown_plugin_bundle_analyzer/src/render_markdown.rs:8` |
| 20.7 KiB |  0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` | `<rolldown::bundle::bundle::Bundle>::bundle_up::{closure#0}` | `crates/rolldown/src/stages/generate_stage/mod.rs:145` |
| 17.5 KiB |  0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/generate_stage/code_splitting.rs:54` |
| 16.1 KiB |  0.1% | `<rolldown_plugin_bundle_analyzer::BundleAnalyzerPlugin>::build_analyze_data` | `<rolldown_plugin_bundle_analyzer::BundleAnalyzerPlugin as rolldown_plugin::plugin::Plugin>::generate_bundle::{closure#0}` | `crates/rolldown_plugin_bundle_analyzer/src/lib.rs:132` |

## Derives

_every impl of the trait, derived or hand-written alike; the total is attribution across many types, not a saving — the ↳ rows are the largest single impls, the ones worth acting on_

|      Size | Share | Derive | Impls |
|----------:|------:|---|------:|
| 254.2 KiB |  1.6% | Debug |  1989 |
|  10.4 KiB |  0.1% | ↳ `<&regress::insn::Insn as core::fmt::Debug>::fmt` |       |
|   7.4 KiB | <0.1% | ↳ `<oxc_resolver::error::ResolveError as core::fmt::Debug>::fmt` |       |
|   2.6 KiB | <0.1% | ↳ `<&[u8; 16] as core::fmt::Debug>::fmt` |       |
|  99.2 KiB |  0.6% | PartialOrd |   127 |
|  33.3 KiB |  0.2% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |       |
|  12.9 KiB |  0.1% | ↳ `core::slice::sort::stable::quicksort::quicksort` |       |
|  10.5 KiB |  0.1% | ↳ `core::slice::sort::shared::smallsort::small_sort_general` |       |
|  65.9 KiB |  0.4% | Clone |    89 |
|  12.5 KiB |  0.1% | ↳ `<rolldown_common::module::normal_module::NormalModule as core::clone::Clone>::clone` |       |
|   8.9 KiB |  0.1% | ↳ `<oxc_resolver::error::ResolveError as core::clone::Clone>::clone` |       |
|   3.0 KiB | <0.1% | ↳ `<oxc_resolver::tsconfig::TsConfig as core::clone::Clone>::clone` |       |
|  28.2 KiB |  0.2% | Deserialize |    34 |
|  12.8 KiB |  0.1% | ↳ `<&mut serde_json::de::Deserializer<serde_json::read::SliceRead> as serde_core::de::Deserializer>::deserialize_struct` |       |
|  11.9 KiB |  0.1% | ↳ `<serde_json::value::Value as serde_core::de::Deserialize>::deserialize` |       |
|   1.1 KiB | <0.1% | ↳ `<serde_json::de::ParserNumber>::visit` |       |
|  20.4 KiB |  0.1% | Default |    52 |
|   7.1 KiB | <0.1% | ↳ `rayon::iter::plumbing::bridge_producer_consumer::helper` |       |
|   2.5 KiB | <0.1% | ↳ `rayon_core::join::join_context::{closure#0}` |       |
|   1.8 KiB | <0.1% | ↳ `<oxc_resolver::options::ResolveOptions as core::default::Default>::default` |       |
|  11.4 KiB |  0.1% | PartialEq |    27 |
|   5.3 KiB | <0.1% | ↳ `<std::path::Components as core::cmp::PartialEq>::eq` |       |
|   1.9 KiB | <0.1% | ↳ `<std::path::PathBuf as core::cmp::PartialEq>::eq` |       |
|   1.2 KiB | <0.1% | ↳ `<std::path::Component as core::cmp::PartialEq>::eq` |       |
|   8.6 KiB |  0.1% | Serialize |     8 |
|   7.3 KiB | <0.1% | ↳ `<serde_json::value::Value as serde_core::ser::Serialize>::serialize` |       |
|   1.3 KiB | <0.1% | ↳ `<alloc::vec::Vec<alloc::string::String> as serde_core::ser::Serialize>::serialize` |       |
|   7.7 KiB | <0.1% | Hash |    12 |
|   5.1 KiB | <0.1% | ↳ `<std::path::Path as core::hash::Hash>::hash` |       |
|   1.3 KiB | <0.1% | ↳ `<std::path::PathBuf as core::hash::Hash>::hash` |       |
|     692 B | <0.1% | ↳ `<&rolldown_common::types::entry_point::EntryPoint as core::hash::Hash>::hash` |       |
|   1.2 KiB | <0.1% | Ord |     4 |
|     800 B | <0.1% | ↳ `<num_bigint::bigint::BigInt as core::cmp::Ord>::cmp` |       |
|     328 B | <0.1% | ↳ `<tracing_subscriber::filter::directive::StaticDirective as core::cmp::Ord>::cmp` |       |
|     136 B | <0.1% | ↳ `<compact_str::CompactString as core::cmp::Ord>::cmp` |       |

## Largest types

_in-memory layout size, not a share of the binary; a large type drives the moves, copies, and drop glue, and an enum is as large as its largest variant, so boxing that one shrinks every value_

|     Size | Type | Layout |
|---------:|---|---|
| 17.2 KiB | `Data` | 56 B padding |
| 14.2 KiB | `State` | 55 B padding |
|  9.7 KiB | `Pow10SignificandTable` | 32 B padding |
|  9.1 KiB | `{async_block_env#1}` | variants 3 9.1 KiB, 4 2.2 KiB, 0 800 B and 2 more; boxing 3 saves ~6.9 KiB per value |
|  8.1 KiB | `Zip<core::array::iter::IntoIter<u8, 8>, core::array::iter::IntoIter<[u32; 256], 8>>` |  |
|  8.0 KiB | `IntoIter<[u32; 256], 8>` |  |
|  8.0 KiB | `PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<[u32; 256]>; 8]>` |  |
|  7.9 KiB | `mi_theap_s` |  |
|  7.8 KiB | `Align64<[u8; 7936]>` |  |
|  7.5 KiB | `{async_fn_env#0}<rolldown_fs::os::OsFileSystem>` | variants 3 7.5 KiB, 4 7.5 KiB, 0 800 B and 2 more; boxing 3 saves ~40 B per value |

## Inlined code

- 6.0 MiB (38.2%) in 1578145 inlined instances, charged to their callers.

### Largest inlined functions

_bytes across every site a function was inlined into, counting only instructions not attributed to a deeper inline_

|      Size | Share | Function | Sites | Defined at |
|----------:|------:|---|------:|---|
| 270.2 KiB |  1.7% | `alloc::alloc::dealloc_nonnull` | 31778 | `library/alloc/src/alloc.rs:127` |
| 103.8 KiB |  0.6% | `core::ptr::copy_nonoverlapping::<u8>` |  6695 | `library/core/src/ptr/mod.rs:528` |
|  99.2 KiB |  0.6% | `rustc_hash::hash_bytes` |   591 | `rustc-hash-2.1.3/src/lib.rs:263` |
|  96.0 KiB |  0.6% | `<mimalloc_safe::MiMalloc as core::alloc::global::GlobalAlloc>::dealloc` | 11202 | `mimalloc-safe-0.1.64/src/lib.rs:60` |
|  83.9 KiB |  0.5% | `<oxc_allocator::arena::Arena>::alloc_layout::{closure#0}` |  3728 | `oxc_allocator-0.146.0/src/arena/alloc_impl.rs:33` |
|  79.3 KiB |  0.5% | `<u8 as core::slice::cmp::SlicePartialEq<u8>>::equal_same_length` |  3577 | `library/core/src/slice/cmp.rs:151` |
|  61.3 KiB |  0.4% | `alloc::alloc::alloc` |  7286 | `library/alloc/src/alloc.rs:95` |
|  53.2 KiB |  0.3% | `core::intrinsics::likely` |  4239 | `library/core/src/intrinsics/mod.rs:435` |
|  43.0 KiB |  0.3% | `alloc::boxed::box_new_uninit` |  2433 | `library/alloc/src/boxed.rs:247` |
|  40.3 KiB |  0.2% | `core::ptr::drop_glue::<alloc::string::String>` | 12811 | `library/core/src/ptr/mod.rs:823` |

### Functions holding the most inlined code

_how much of each body is other functions' instances — splitting the function or un-inlining the callees works here_

|     Size | Share | Of its body | Function |
|---------:|------:|------------:|---|
| 22.8 KiB |  0.1% |         51% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |
| 20.0 KiB |  0.1% |         64% | `core::ptr::drop_glue::<hashbrown::scopeguard::ScopeGuard<&mut hashbrown::raw::RawTableInner, <hashbrown::raw::RawTableInner>::rehash_in_place::{closure#0}>>` |
| 16.5 KiB |  0.1% |         61% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |
| 15.1 KiB |  0.1% |         49% | `<rolldown::module_finalizers::ScopeHoistingFinalizer as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_program` |
| 14.9 KiB |  0.1% |         57% | `<rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}` |
| 14.3 KiB |  0.1% |         63% | `napi_sys::functions::napi1::load` |
| 13.4 KiB |  0.1% |         65% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` |
| 13.2 KiB |  0.1% |         36% | `rolldown_binding::utils::normalize_binding_options::normalize_binding_options` |
| 13.1 KiB |  0.1% |         63% | `rolldown_plugin_bundle_analyzer::render_markdown::render_markdown` |
| 12.9 KiB |  0.1% |         44% | `rolldown::utils::prepare_build_context::prepare_build_context` |

### By the crate the inlined code came from

_the footprint of code that vanished into its callers, credited to the defining crate — the by-crate table above counts only surviving symbols_

|      Size | Share | Crate | Instances |
|----------:|------:|---|----------:|
|   3.0 MiB | 18.7% | `core` |    513234 |
| 724.7 KiB |  4.5% | `alloc` |    183940 |
| 321.3 KiB |  2.0% | `oxc_ast_visit` |     21930 |
| 140.8 KiB |  0.9% | `rolldown` |      9752 |
| 133.0 KiB |  0.8% | `mimalloc_safe` |     14277 |
| 126.2 KiB |  0.8% | `hashbrown` |     22266 |
| 121.1 KiB |  0.7% | `napi` |      3239 |
| 109.7 KiB |  0.7% | `oxc_allocator` |     37254 |
| 109.1 KiB |  0.7% | `rustc_hash` |      8227 |
|  94.0 KiB |  0.6% | `rolldown_binding` |      1228 |

### Workspace lines that pulled in the most inlined code

|      Size | Share | Line | Inlined | Source |
|----------:|------:|---|--------:|---|
| 133.0 KiB |  0.8% | `crates/rolldown_binding/src/lib.rs:36` |   14277 | `static ALLOC: mimalloc_safe::MiMalloc = mimalloc_safe::MiMalloc;` |
|  20.0 KiB |  0.1% | `crates/rolldown_binding/src/types/binding_magic_string.rs:443` |     567 | `#[napi]` |
|  18.7 KiB |  0.1% | `crates/rolldown_binding/src/types/binding_normalized_options.rs:21` |     497 | `#[napi]` |
|   9.0 KiB |  0.1% | `crates/rolldown_binding/src/types/js_callback.rs:188` |      80 | `match self.call_async_catch(args).await? {` |
|   8.1 KiB | <0.1% | `crates/rolldown_binding/src/types/js_callback.rs:121` |      94 | `match self.call_async_catch(args).await? {` |
|   6.9 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_output_chunk.rs:17` |     203 | `#[napi]` |
|   6.2 KiB | <0.1% | `crates/rolldown_binding/src/options/plugin/binding_plugin_context.rs:24` |     190 | `#[napi]` |
|   5.8 KiB | <0.1% | `crates/rolldown_binding/src/binding_dev_engine.rs:31` |     195 | `#[napi]` |
|   4.6 KiB | <0.1% | `crates/rolldown_binding/src/options/binding_output_options/mod.rs:38` |     213 | `#[napi_derive::napi(object, object_to_js = false)]` |
|   4.4 KiB | <0.1% | `crates/rolldown_binding/src/options/plugin/binding_plugin_options.rs:39` |     251 | `#[napi_derive::napi(object, object_to_js = false)]` |

## Assembly

- 2.6 MiB (16.6%) of code in 5457 functions with assembly, 687033 instructions, 4.0 B each; from `/Users/boshen/github/rolldown/rolldown/target/bsize/release/deps/rolldown_binding.s`.
- 302 more functions in the assembly never reached the binary.
- `~` sizes below are instruction counts converted at that rate.

### Identical function bodies

_the same instructions under different names; a linker folding identical code keeps one, and so does instantiating one — 24.6 KiB recoverable from 153 groups (367 functions)_

| Recoverable | Share | Functions | Copies |    Each |
|------------:|------:|---|-------:|--------:|
|     1.5 KiB | <0.1% | `napi::js_values::deferred::napi_resolve_deferred::<napi::js_values::unknown::Unknown, napi::tokio_runtime::execute_future_impl<(), <rolldown_binding::options::plugin::binding_callable_builtin_plugin::BindingCallableBuiltinPlugin>::watch_change::{closure#0}, <napi::tokio_runtime::AsyncBlockBuilder<(), <rolldown_binding::options::plugin::binding_callable_builtin_plugin::BindingCallableBuiltinPlugin>::watch_change::{closure#0}, <rolldown_binding::options::plugin::binding_callable_builtin_plugin::BindingCallableBuiltinPlugin>::watch_change::{closure#1}>>::build::{closure#0}, napi::error::Error>::{closure#0}::{closure#0}>` ≡ `napi::js_values::deferred::napi_resolve_deferred::<napi::js_values::unknown::Unknown, napi::tokio_runtime::execute_future_impl<(), core::pin::Pin<alloc::boxed::Box<dyn core::future::future::Future<Output = core::result::Result<(), napi::error::Error>> + core::marker::Send>>, <napi::env::Env>::spawn_future<(), core::pin::Pin<alloc::boxed::Box<dyn core::future::future::Future<Output = core::result::Result<(), napi::error::Error>> + core::marker::Send>>>::{closure#0}, napi::error::Error>::{closure#0}::{closure#0}>` |      2 | 1.5 KiB |
|     1.2 KiB | <0.1% | `<hashbrown::raw::RawTable<usize>>::reserve_rehash::<indexmap::inner::get_hash<alloc::string::String, rolldown_binding::types::binding_rendered_chunk::BindingRenderedChunk>::{closure#0}>` ≡ `<hashbrown::raw::RawTable<usize>>::reserve_rehash::<indexmap::inner::get_hash<rolldown_common::types::symbol_ref::SymbolRef, alloc::vec::Vec<oxc_str::compact_str::CompactStr>>::{closure#0}>` |      2 | 1.2 KiB |
|       880 B | <0.1% | `core::ptr::drop_glue::<hashbrown::raw::RawTable<(rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::vec::Vec<(rolldown_common::types::import_record::ImportRecordIdx, rolldown_common::ecmascript::module_idx::ModuleIdx)>)>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::HashMap<oxc_syntax::symbol::SymbolId, alloc::vec::Vec<oxc_syntax::scope::ScopeId>, rustc_hash::FxBuildHasher>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::HashMap<oxc_transformer::common::helper_loader::Helper, alloc::string::String, rustc_hash::FxBuildHasher>>` ≡ 2 more |      5 |   220 B |
|       680 B | <0.1% | `core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<alloc::string::String>>` ≡ `core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::path::PathBuf>>` ≡ `core::ptr::drop_glue::<core::iter::adapters::enumerate::Enumerate<alloc::vec::into_iter::IntoIter<alloc::string::String>>>` ≡ 3 more |      6 |   136 B |
|       560 B | <0.1% | `core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>` ≡ `core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<rolldown_common::types::ins_chunk_idx::InsChunkIdx>>>` ≡ `core::ptr::drop_glue::<alloc::vec::Vec<oxc_resolver::tsconfig::ProjectReference>>` ≡ 3 more |      6 |   112 B |
|       456 B | <0.1% | `core::ptr::drop_glue::<indexmap::map::IndexMap<(rolldown_common::ecmascript::module_idx::ModuleIdx, rolldown_common::types::import_record::ImportRecordIdx), alloc::string::String, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>>` ≡ `core::ptr::drop_glue::<indexmap::map::IndexMap<alloc::string::String, rolldown_common::types::watch::WatcherChangeKind, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>>` ≡ `core::ptr::drop_glue::<indexmap::map::IndexMap<rolldown_common::types::chunk_idx::ChunkIdx, alloc::vec::Vec<rolldown_common::chunk::types::cross_chunk_import_item::CrossChunkImportItem>, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>>` ≡ 1 more |      4 |   152 B |
|       440 B | <0.1% | `core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, u32)>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::HashMap<alloc::string::String, u32, rustc_hash::FxBuildHasher>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::HashMap<rolldown_utils::bitset::BitSet, rolldown_common::types::chunk_idx::ChunkIdx, rustc_hash::FxBuildHasher>>` |      3 |   220 B |
|       408 B | <0.1% | `core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<oxc_transformer::common::helper_loader::Helper, alloc::string::String>, <rolldown_binding::options::binding_transform_options::BindingEnhancedTransformResult>::from_enhanced_transform_result::{closure#0}>>` ≡ `core::ptr::drop_glue::<hashbrown::raw::RawIntoIter<(oxc_transformer::common::helper_loader::Helper, alloc::string::String)>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::IntoIter<rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::vec::Vec<(rolldown_common::ecmascript::module_idx::ModuleIdx, rolldown_common::types::stmt_info::StmtInfoIdx, oxc_syntax::node::NodeId, rolldown_common::types::import_record::ImportRecordIdx)>>>` |      3 |   204 B |
|       408 B | <0.1% | `core::ptr::drop_glue::<indexmap::Bucket<alloc::string::String, alloc::string::String>>` ≡ `core::ptr::drop_glue::<napi::bindgen_runtime::js_values::function::FnArgs<(alloc::string::String, alloc::string::String)>>` ≡ `core::ptr::drop_glue::<oxc_resolver::error::JSONError>` ≡ 4 more |      7 |    68 B |
|       348 B | <0.1% | `core::ptr::drop_glue::<tokio::loom::std::mutex::Mutex<tokio::sync::broadcast::Slot<()>>>` ≡ `core::ptr::drop_glue::<tokio::loom::std::mutex::Mutex<tokio::sync::broadcast::Tail>>` ≡ `core::ptr::drop_glue::<tokio::sync::notify::Notify>` ≡ 1 more |      4 |   116 B |

### Panic call sites

_each is a compare, a branch, and a cold block that loads the location and calls; the location is 24 B more of read-only data_

- ~127.7 KiB (0.8%) in the blocks of 14040 sites: 311 bounds checks, 746 unwraps, 3124 allocation failures, 9859 other; 927 distinct locations and messages loaded by them.

|    ~Size | Share | Function | Sites | Instructions |
|---------:|------:|---|------:|-------------:|
| ~2.4 KiB | <0.1% | `rolldown_binding::types::binding_normalized_options::__napi_impl_helper_BindingNormalizedOptions_16::__napi_register__BindingNormalizedOptions_impl_285::_::__CTOR_PRIVATE_REF::__ctor_private` |    84 |          606 |
| ~2.3 KiB | <0.1% | `rolldown_binding::types::binding_magic_string::__napi_impl_helper_BindingMagicString_14::__napi_register__BindingMagicString_impl_240::_::__CTOR_PRIVATE_REF::__ctor_private` |    82 |          592 |
| ~1.3 KiB | <0.1% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |   173 |          340 |
|  ~1012 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunk_name_and_preliminary_filenames::{closure#0}::{closure#0}` |    77 |          253 |
|  ~1008 B | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |    99 |          252 |
|   ~904 B | <0.1% | `<rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}` |    94 |          226 |
|   ~900 B | <0.1% | `rolldown::utils::chunk::finalize_chunks::finalize_assets::{closure#0}::{closure#0}` |    64 |          225 |
|   ~868 B | <0.1% | `rolldown_binding::types::binding_output_chunk::__napi_impl_helper_BindingOutputChunk_18::__napi_register__BindingOutputChunk_impl_312::_::__CTOR_PRIVATE_REF::__ctor_private` |    35 |          217 |
|   ~844 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` |    58 |          211 |
|   ~768 B | <0.1% | `<rolldown::stages::scan_stage::ScanStage<rolldown_fs::os::OsFileSystem>>::scan::{closure#0}::{closure#0}` |    78 |          192 |

### Formatting call sites

_calls into core::fmt and alloc::fmt; the block before each builds the Arguments_

- ~43.8 KiB (0.3%) in the blocks of 1039 sites.

|    ~Size | Share | Function | Sites | Instructions |
|---------:|------:|---|------:|-------------:|
| ~1.1 KiB | <0.1% | `<rolldown_plugin_vite_reporter::ViteReporterPlugin as rolldown_plugin::plugin::Plugin>::write_bundle::{closure#0}` |    18 |          274 |
|   ~844 B | <0.1% | `<rolldown_plugin_vite_resolve::vite_resolve_plugin::ViteResolvePlugin as rolldown_plugin::plugin::Plugin>::resolve_id::{closure#0}` |    17 |          211 |
|   ~672 B | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |    11 |          168 |
|   ~516 B | <0.1% | `<owo_colors::dyn_styles::Style>::fmt_prefix` |    22 |          129 |
|   ~500 B | <0.1% | `<oxc_resolver::error::ResolveError as core::fmt::Debug>::fmt` |     9 |          125 |
|   ~428 B | <0.1% | `<napi::status::Status as core::fmt::Debug>::fmt` |    22 |          107 |
|   ~428 B | <0.1% | `<&[u8; 16] as core::fmt::Debug>::fmt` |    18 |          107 |
|   ~404 B | <0.1% | `<&[u8; 15] as core::fmt::Debug>::fmt` |    17 |          101 |
|   ~400 B | <0.1% | `<rolldown_binding::options::binding_output_options::binding_manual_code_splitting_options::BindingMatchGroup as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |     6 |          100 |
|   ~380 B | <0.1% | `<&[u8; 14] as core::fmt::Debug>::fmt` |    16 |           95 |

### Values copied through memory

_runs of 8 or more loads and stores back to back, and calls to memcpy for anything larger; boxing the value or passing it by reference removes them_

- ~146.1 KiB (0.9%) in 2913 runs, 37397 instructions, plus 1859 memcpy-family calls.

|    ~Size | Share | Function | Instructions | Runs | Calls |
|---------:|------:|---|-------------:|-----:|------:|
| ~4.1 KiB | <0.1% | `rolldown_binding::utils::normalize_binding_options::normalize_binding_options` |         1046 |   80 |     8 |
| ~3.1 KiB | <0.1% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |          799 |   66 |    37 |
| ~3.1 KiB | <0.1% | `rolldown_binding::types::binding_normalized_options::__napi_impl_helper_BindingNormalizedOptions_16::__napi_register__BindingNormalizedOptions_impl_285::_::__CTOR_PRIVATE_REF::__ctor_private` |          797 |   49 |     0 |
| ~2.9 KiB | <0.1% | `rolldown_binding::types::binding_magic_string::__napi_impl_helper_BindingMagicString_14::__napi_register__BindingMagicString_impl_240::_::__CTOR_PRIVATE_REF::__ctor_private` |          755 |   52 |     0 |
| ~2.3 KiB | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` |          578 |   34 |    23 |
| ~2.1 KiB | <0.1% | `rolldown::ecmascript::ecma_module_view_factory::create_ecma_view::{closure#0}` |          532 |   33 |    16 |
| ~2.0 KiB | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |          522 |   36 |    14 |
| ~1.7 KiB | <0.1% | `<rolldown_binding::options::plugin::js_plugin::JsPlugin as rolldown_plugin::plugin::Plugin>::load::{closure#0}` |          446 |   19 |     6 |
| ~1.7 KiB | <0.1% | `<rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}` |          438 |   31 |    21 |
| ~1.4 KiB | <0.1% | `rolldown::module_loader::resolve_utils::resolve_dependencies::<rolldown_fs::os::OsFileSystem>::{closure#0}` |          348 |   28 |    14 |

### Workspace lines compiled to the most instructions

_the line an instruction came from, after inlining, every instantiation summed_

|     ~Size | Share | Line | Instructions | Source |
|----------:|------:|---|-------------:|---|
| ~16.8 KiB |  0.1% | `crates/rolldown_binding/src/types/binding_magic_string.rs:443` |         4293 | `#[napi]` |
| ~10.9 KiB |  0.1% | `crates/rolldown_binding/src/types/binding_normalized_options.rs:21` |         2796 | `#[napi]` |
|  ~5.2 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_output_chunk.rs:17` |         1335 | `#[napi]` |
|  ~5.2 KiB | <0.1% | `crates/rolldown_binding/src/binding_dev_engine.rs:31` |         1330 | `#[napi]` |
|  ~5.1 KiB | <0.1% | `crates/rolldown_binding/src/utils/normalize_binding_options.rs:750` |         1295 | `};` |
|  ~4.3 KiB | <0.1% | `crates/rolldown_binding/src/options/plugin/binding_plugin_context.rs:24` |         1099 | `#[napi]` |
|  ~3.7 KiB | <0.1% | `crates/rolldown_binding/src/options/binding_output_options/mod.rs:38` |          948 | `#[napi_derive::napi(object, object_to_js = false)]` |
|  ~3.4 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_module_info.rs:8` |          865 | `#[napi]` |
|  ~2.8 KiB | <0.1% | `crates/rolldown_binding/src/types/js_callback.rs:187` |          721 | `async fn await_call(&self, args: Args) -> Result<Ret, napi::Error> {` |
|  ~2.8 KiB | <0.1% | `crates/rolldown_binding/src/options/plugin/binding_plugin_options.rs:39` |          707 | `#[napi_derive::napi(object, object_to_js = false)]` |

## Constant data

_every constant the assembly spells out, sized from its directives and read by shape; only what a linked function reaches counts, and under lto="fat" that is the whole program_

- 151.1 KiB (0.9%) in 4894 constants a linked function reaches (5401 defined), against 1.9 MiB of read-only data sections.

### By kind

|     Size | Share | Kind | Constants |
|---------:|------:|---|----------:|
| 40.3 KiB |  0.2% | vtables |       929 |
| 27.9 KiB |  0.2% | type, variant, and field names |      1802 |
| 20.7 KiB |  0.1% | panic locations (24 B records) |       885 |
| 17.5 KiB |  0.1% | source paths (what panic locations point at) |       228 |
| 17.2 KiB |  0.1% | messages and other text |       395 |
| 16.2 KiB |  0.1% | byte tables |       474 |
|  5.1 KiB | <0.1% | other pointer data |        66 |
|  1.8 KiB | <0.1% | lookup tables for `match` |        32 |
|  1.6 KiB | <0.1% | function pointer tables |        34 |
|  1.4 KiB | <0.1% | jump tables for `match` |        32 |
|    896 B | <0.1% | string slices and format pieces |        11 |
|    576 B | <0.1% | tables of slices and records |         6 |
|  5.7 KiB | <0.1% | of the text: loaded only on the way to a panic (messages and their pieces) |       126 |

### Panic locations

_a 24 B record per panic site — an unwrap, an index, an expect — plus the source path once: 885 records in all, 38.2 KiB_

|  Size | Share | Workspace file | Records | Most on lines |
|------:|------:|---|--------:|---|
| 709 B | <0.1% | `crates/rolldown/src/bundle/bundle.rs` |      28 | 43, 45, 47, 49, 53 |
| 729 B | <0.1% | `crates/rolldown_plugin/src/plugin_driver/output_hooks.rs` |      28 | 17, 22, 38, 46, 58 |
| 704 B | <0.1% | `crates/rolldown_plugin/src/plugin_driver/build_hooks.rs` |      27 | 45, 50, 80, 89, 110 |
| 641 B | <0.1% | `crates/rolldown_binding/src/options/plugin/parallel_js_plugin.rs` |      24 | 49, 53, 55, 68, 91 |
| 603 B | <0.1% | `crates/rolldown/src/module_loader/module_loader.rs` |      23 | 1163, 354, 366, 379, 384 |
| 608 B | <0.1% | `crates/rolldown_binding/src/options/plugin/js_plugin.rs` |      23 | 89, 109, 168, 202, 240 |
| 589 B | <0.1% | `crates/rolldown_plugin/src/plugin.rs` |      23 | 44, 56, 71, 83, 95 |
| 447 B | <0.1% | `crates/rolldown_binding/src/utils/normalize_binding_options.rs` |      16 | 77, 98, 115, 136, 156 |
| 432 B | <0.1% | `crates/rolldown_plugin_vite_reporter/src/lib.rs` |      16 | 245, 51, 58, 70, 82 |
| 377 B | <0.1% | `crates/rolldown/src/utils/load_source.rs` |      14 | 180, 191, 22, 66, 67 |

|  Size | Share | Function loading the most records | Records |
|------:|------:|---|--------:|
| 528 B | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |      22 |
| 504 B | <0.1% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |      21 |
| 432 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunk_name_and_preliminary_filenames::{closure#0}::{closure#0}` |      18 |
| 432 B | <0.1% | `<rolldown_watcher::watch_coordinator::WatchCoordinator<rolldown_binding::watcher::NapiWatcherEventHandler>>::run::{closure#0}` |      18 |
| 384 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` |      16 |
| 360 B | <0.1% | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |      15 |
| 360 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::instantiate_chunks::{closure#0}::{closure#0}` |      15 |
| 360 B | <0.1% | `<rolldown::stages::scan_stage::ScanStage<rolldown_fs::os::OsFileSystem>>::scan::{closure#0}::{closure#0}` |      15 |
| 360 B | <0.1% | `rolldown::utils::chunk::finalize_chunks::finalize_assets::{closure#0}::{closure#0}` |      15 |
| 360 B | <0.1% | `rolldown_utils::futures::block_on::<rolldown_utils::futures::block_on_spawn_all<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::string::String>, <rolldown_plugin_vite_dynamic_import_vars::ViteDynamicImportVarsPlugin as rolldown_plugin::plugin::Plugin>::transform::{closure#0}::{closure#0}>, core::option::Option<alloc::string::String>>::{closure#0}>::{closure#0}` |      15 |

### Largest strings

|  Size | Share | String | Kind | Loaded by |
|------:|------:|---|---|---|
| 223 B | <0.1% | `"The id starts with \"virtual:\", which by convention denotes a vir…"` | message | `rolldown::module_loader::resolve_utils::resolve_dependencies::<rolldown_fs::os::OsFileSystem>::{closure#0}` |
| 196 B | <0.1% | `` "This MagicString was already passed to `sendMagicString()`, whic…" `` | message | `<rolldown_binding::types::binding_magic_string::BindingMagicString>::trim_start` and 33 more |
| 193 B | <0.1% | `"Cannot start a runtime from within a runtime. This happens becau…"` | message | `rolldown_utils::futures::block_on::<rolldown_utils::futures::block_on_spawn_all<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::string::String>, <rolldown_plugin_vite_dynamic_import_vars::ViteDynamicImportVarsPlugin as rolldown_plugin::plugin::Plugin>::transform::{closure#0}::{closure#0}>, core::option::Option<alloc::string::String>>::{closure#0}>::{closure#0}` and 2 more |
| 185 B | <0.1% | `` "Encountered a module with type `asset`, but no plugin handled it…" `` | message | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |
| 182 B | <0.1% | `` "Encountered a module with type `copy`, but no plugin handled it.…" `` | message | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |
| 172 B | <0.1% | `` "The `PluginContext.load` only work at `resolveId/load/transform/…" `` | message | `<rolldown_plugin::plugin_context::native_plugin_context::NativePluginContextImpl>::load::{closure#0}` |
| 159 B | <0.1% | `` "Maybe you expected `resolve.alias` to call other plugins resolve…" `` | message | `rolldown::module_loader::resolve_utils::resolve_dependencies::<rolldown_fs::os::OsFileSystem>::{closure#0}` |
| 152 B | <0.1% | `` "Memory has been freed by `freeExternalMemory()`. Cannot access p…" `` | message | `<rolldown_binding::types::binding_output_chunk::BindingOutputChunk>::get_exports` and 19 more |
| 142 B | <0.1% | `"Bundling CSS is no longer supported (experimental support has be…"` | message | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::run::{closure#0}::{closure#0}` |
| 141 B | <0.1% | `"TransformPluginContext: failed to send MagicString to sourcemap …"` | message | `<rolldown_binding::options::plugin::binding_transform_context::BindingTransformPluginContext>::send_magic_string` |

### Lookup and jump tables

_by the function whose match built them_

|  Size | Share | Function | Lookup | Jump |
|------:|------:|---|-------:|-----:|
| 464 B | <0.1% | `<rolldown_binding::options::binding_transform_options::BindingEnhancedTransformResult>::from_enhanced_transform_result` |      2 |    0 |
| 288 B | <0.1% | `<rolldown_binding::options::plugin::types::binding_builtin_plugin_name::BindingBuiltinPluginName as core::fmt::Debug>::fmt` |      2 |    0 |
| 192 B | <0.1% | `<rolldown_binding::options::plugin::types::binding_filter_expression::FilterTokenKind as core::fmt::Debug>::fmt` |      2 |    0 |
| 144 B | <0.1% | `<rolldown_plugin_isolated_declaration::type_import_visitor::TypeImportVisitor as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_statement` |      0 |    1 |
| 144 B | <0.1% | `<rolldown_plugin_vite_dynamic_import_vars::ast_visit::DynamicImportVarsVisit as oxc_ast_visit::generated::visit_js::VisitJs>::visit_statement` |      0 |    1 |
| 144 B | <0.1% | `oxc_traverse::generated::walk::walk_statement::<(), rolldown::hmr::hmr_ast_finalizer::HmrAstFinalizer>` |      0 |    1 |
| 112 B | <0.1% | `<rolldown_common::types::module_def_format::ModuleDefFormat as core::fmt::Debug>::fmt` |      2 |    0 |
| 102 B | <0.1% | `<rolldown_plugin_isolated_declaration::type_import_visitor::TypeImportVisitor as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_expression` |      0 |    1 |
| 102 B | <0.1% | `oxc_traverse::generated::walk::walk_expression::<(), rolldown::hmr::hmr_ast_finalizer::HmrAstFinalizer>` |      0 |    1 |
|  96 B | <0.1% | `<&core::num::error::IntErrorKind as core::fmt::Debug>::fmt` |      2 |    0 |

### Functions carrying the most constant data

_ranked by what only that function reaches, directly or through a table's pointers — what rewriting it alone frees; the total counts shared constants too_

| Exclusive | Share | Function | Constants |   Total |
|----------:|------:|---|----------:|--------:|
|   7.7 KiB | <0.1% | `<alloc::sync::Arc<dyn rolldown_plugin::pluginable::Pluginable> as core::convert::TryFrom<rolldown_binding::options::plugin::binding_builtin_plugin::BindingBuiltinPlugin>>::try_from` |        32 | 7.7 KiB |
|   1.4 KiB | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |        70 | 2.1 KiB |
|   1.4 KiB | <0.1% | `<rolldown_plugin_vite_module_preload_polyfill::ViteModulePreloadPolyfillPlugin as rolldown_plugin::plugin::Plugin>::load::{closure#0}` |         4 | 1.4 KiB |
|   1.2 KiB | <0.1% | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |        36 | 1.7 KiB |
|   1.1 KiB | <0.1% | `<&regress::insn::Insn as core::fmt::Debug>::fmt` |        85 | 1.5 KiB |
|   1.0 KiB | <0.1% | `<rolldown_binding::options::plugin::binding_plugin_options::BindingPluginOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |        88 | 1.1 KiB |
|     979 B | <0.1% | `<rolldown_plugin_vite_resolve::vite_resolve_plugin::ViteResolvePlugin as rolldown_plugin::plugin::Plugin>::resolve_id::{closure#0}` |        31 | 1.3 KiB |
|     978 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` |        63 | 1.9 KiB |
|     941 B | <0.1% | `<rolldown_binding::options::binding_transform_options::BindingEnhancedTransformResult>::from_enhanced_transform_result` |        33 |   981 B |
|     909 B | <0.1% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |        79 | 3.2 KiB |

## Dynamic relocations

_every pointer kept in data is a slot the loader fills at start; the records are compressed, so the slot's own 8 B is the cost — offsets instead of pointers remove it_

- 10.3 KiB (0.1%) in relocation records for 38180 pointer slots (298.3 KiB of slots).
