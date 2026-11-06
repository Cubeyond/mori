# Dependency notices

Mori's own code is licensed under Elastic-2.0. These files preserve third-party rights; they do not relicense upstream code under Mori's terms.

`cargo.json` records 296 package records selected by cargo-bundle-licenses 5.0.0. It is generated license data, not hand-authored application code. The catalog can include conditional-platform and build dependencies. `cargo.sha256` binds this reviewed catalog to the exact Cargo.lock so dependency changes cannot silently ship stale notices. The container carries a compressed copy without changing its contents.

To review a Cargo dependency update, regenerate from the server directory with:

```sh
cargo install cargo-bundle-licenses --version 5.0.0 --locked
cd server
cargo bundle-licenses --format json --output ../licenses/cargo-next.json --previous ../licenses/cargo.json
```

Review all new or missing texts before replacing cargo.json and refreshing cargo.sha256. The generator is an aid; its confidence warnings and NOT FOUND markers are not legal verification. The current catalog's eight missing texts were reviewed: base64-simd/vsimd use the exact upstream monorepo MIT text; valuable uses its exact upstream MIT text; crc-catalog uses the files in its published LICENSES directory; r-efi explicitly offers MIT as an alternative, selected here with its exact upstream source reference and standard MIT permission text because its published package omits separate root license files.

Reviewed upstream sources:

- https://github.com/Nugine/simd/blob/d74c030d9dc4f3cae02146d1f497ff62726ef09a/LICENSE
- https://github.com/tokio-rs/valuable/blob/9efc29b6e58cef28f6566a47aa7e142a55fead77/LICENSE
- https://github.com/r-efi/r-efi/tree/7e1b0322d31d625f81a5656096330934f9cd835d

The web build generates its notices from `pnpm licenses list --prod --json`, preserving complete published LICENSE/COPYING/NOTICE files. Exact-version omissions for is-reference and locate-character are recorded in web/scripts/legal/overrides.json, with the upstream MIT declarations and standard permission text. Missing notices fail the build. Only package names, versions, upstream attribution and terms are distributed; local install paths are removed.

Mediabunny remains MPL-2.0. Its published npm archive includes its preferred TypeScript source form; the notice record links the exact archive. MPL terms remain intact for that component. The copied shadcn-svelte code/styles remain under their MIT notices; font and theme notices are distributed with the static assets and container.

This inventory preserves attribution and identifies declared license choices. It is not a claim that an automated scanner establishes ownership or guarantees a legal outcome.

Historical SQL migrations remain byte-identical so SQLx checksum validation is preserved. Their first-party licensing is covered by the root NOTICE, alongside generated files that cannot retain handwritten headers.
