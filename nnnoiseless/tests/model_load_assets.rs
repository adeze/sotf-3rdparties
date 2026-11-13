//! Real-asset oracles for the checked `.rnnn` loader.
//!
//! The staged legacy weight files are sealed against root's manifest hashes,
//! parsed for exact graph facts, and verified against an independent
//! column-major transpose oracle. Expected hashes and graph facts come from
//! `source-manifest.json` and the compatibility report — not from this
//! implementation — so a mismatch here is a real asset or loader finding.

// Rust guideline compliant 2026-02-21

use nnnoiseless::{parse_rnnn_model, Activation, DenoiseState, RnnModel};
use std::borrow::Cow;

const LQ_RNNN: &[u8] = include_bytes!(
    "../../../sotf-daw/crates/sotf-plugins/crates/plugins-denoiser/models/legacy-rnnoise-nu/leavened-quisling-2018-08-31/lq.rnnn"
);
const SH_RNNN: &[u8] = include_bytes!(
    "../../../sotf-daw/crates/sotf-plugins/crates/plugins-denoiser/models/legacy-rnnoise-nu/somnolent-hogwash-2018-09-01/sh.rnnn"
);

/// Root-pinned SHA-256 from `source-manifest.json` (upstream commit 3eee541).
const LQ_SHA256: &str = "1957528b752799fddf06270bc5469af7cf54c3badc358544ae2abed730943ff9";
const SH_SHA256: &str = "70bb6685eb0c2a1d18e2918dca3fbfbd39317010b1802eb1b6ea73a92f3fdec0";
/// FIPS 180-4 empty-string vector; self-checks the hasher below.
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Minimal SHA-256 (FIPS 180-4, 32-bit words, big-endian).
///
/// No hash dependency is available to this crate, so the asset seal carries
/// its own implementation. It is validated against the well-known
/// empty-string vector before any asset hash is trusted.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut padded = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    for block in padded.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut hex = String::with_capacity(64);
    for word in h {
        hex.push_str(&format!("{word:08x}"));
    }
    hex
}

fn weight_arrays(model: &RnnModel) -> Vec<(&'static str, &Cow<'static, [i8]>)> {
    vec![
        ("input_dense/bias", &model.input_dense.bias),
        (
            "input_dense/input_weights",
            &model.input_dense.input_weights,
        ),
        ("vad_gru/bias", &model.vad_gru.bias),
        ("vad_gru/input_weights", &model.vad_gru.input_weights),
        (
            "vad_gru/recurrent_weights",
            &model.vad_gru.recurrent_weights,
        ),
        ("noise_gru/bias", &model.noise_gru.bias),
        ("noise_gru/input_weights", &model.noise_gru.input_weights),
        (
            "noise_gru/recurrent_weights",
            &model.noise_gru.recurrent_weights,
        ),
        ("denoise_gru/bias", &model.denoise_gru.bias),
        (
            "denoise_gru/input_weights",
            &model.denoise_gru.input_weights,
        ),
        (
            "denoise_gru/recurrent_weights",
            &model.denoise_gru.recurrent_weights,
        ),
        ("denoise_output/bias", &model.denoise_output.bias),
        (
            "denoise_output/input_weights",
            &model.denoise_output.input_weights,
        ),
        ("vad_output/bias", &model.vad_output.bias),
        ("vad_output/input_weights", &model.vad_output.input_weights),
    ]
}

#[test]
fn staged_assets_match_pinned_manifest_hashes() {
    assert_eq!(sha256_hex(b""), EMPTY_SHA256);
    assert_eq!(LQ_RNNN.len(), 297_041);
    assert_eq!(SH_RNNN.len(), 297_646);
    assert_eq!(sha256_hex(LQ_RNNN), LQ_SHA256);
    assert_eq!(sha256_hex(SH_RNNN), SH_SHA256);
}

#[test]
fn staged_assets_parse_with_exact_graph_facts() {
    for (name, bytes) in [("lq", LQ_RNNN), ("sh", SH_RNNN)] {
        let model = parse_rnnn_model(bytes).unwrap();
        assert_eq!(
            (model.input_dense.nb_inputs, model.input_dense.nb_neurons),
            (42, 24)
        );
        assert_eq!(
            (model.vad_gru.nb_inputs, model.vad_gru.nb_neurons),
            (24, 24)
        );
        assert_eq!(
            (model.noise_gru.nb_inputs, model.noise_gru.nb_neurons),
            (90, 48)
        );
        assert_eq!(
            (model.denoise_gru.nb_inputs, model.denoise_gru.nb_neurons),
            (114, 96)
        );
        assert_eq!(
            (
                model.denoise_output.nb_inputs,
                model.denoise_output.nb_neurons
            ),
            (96, 22)
        );
        assert_eq!(
            (model.vad_output.nb_inputs, model.vad_output.nb_neurons),
            (24, 1)
        );
        // File activations are honored, including Tanh GRUs that differ from
        // the bundled model's Relu.
        assert_eq!(model.input_dense.activation, Activation::Tanh, "{name}");
        assert_eq!(model.vad_gru.activation, Activation::Tanh, "{name}");
        assert_eq!(model.noise_gru.activation, Activation::Relu, "{name}");
        assert_eq!(model.denoise_gru.activation, Activation::Tanh, "{name}");
        assert_eq!(
            model.denoise_output.activation,
            Activation::Sigmoid,
            "{name}"
        );
        assert_eq!(model.vad_output.activation, Activation::Sigmoid, "{name}");
        let total: usize = weight_arrays(&model).iter().map(|(_, a)| a.len()).sum();
        assert_eq!(total, 87_503, "{name}");
        let nonzero: usize = weight_arrays(&model)
            .iter()
            .map(|(_, a)| a.iter().filter(|w| **w != 0).count())
            .sum();
        println!("{name}: {nonzero} nonzero of {total} weights");
        assert!(nonzero > 0, "{} weights are all zero", name);
    }
}

#[test]
fn staged_assets_parse_to_distinct_weight_sets() {
    let lq = parse_rnnn_model(LQ_RNNN).unwrap();
    let sh = parse_rnnn_model(SH_RNNN).unwrap();
    let mut differ = 0;
    let mut total = 0;
    for ((name, a), (_, b)) in weight_arrays(&lq).iter().zip(weight_arrays(&sh).iter()) {
        assert_eq!(a.len(), b.len(), "{name}");
        total += a.len();
        differ += a.iter().zip(b.iter()).filter(|(x, y)| x != y).count();
    }
    println!("lq-vs-sh: {differ} differing of {total} weights");
    assert!(differ > 0, "staged assets parsed identically");
}

fn check_matrix(
    tokens: &mut std::str::SplitWhitespace<'_>,
    parsed: &[i8],
    m: usize,
    n: usize,
    context: &str,
) {
    for j in 0..m {
        for i in 0..n {
            let raw: i32 = tokens.next().unwrap().parse().unwrap();
            assert_eq!(parsed[i * m + j], raw as i8, "{context} i={i} j={j}");
        }
    }
}

fn check_bias(tokens: &mut std::str::SplitWhitespace<'_>, parsed: &[i8], context: &str) {
    for (index, value) in parsed.iter().enumerate() {
        let raw: i32 = tokens.next().unwrap().parse().unwrap();
        assert_eq!(*value, raw as i8, "{context}[{index}]");
    }
}

fn skip_header(tokens: &mut std::str::SplitWhitespace<'_>) {
    for _ in 0..3 {
        tokens.next().unwrap();
    }
}

#[test]
fn staged_transpose_matches_column_major_file_order() {
    for (name, bytes) in [("lq", LQ_RNNN), ("sh", SH_RNNN)] {
        let model = parse_rnnn_model(bytes).unwrap();
        // Independent re-walk: file order is input-major (j outer, i inner).
        let text = std::str::from_utf8(bytes).unwrap();
        let body = text.split_once('\n').unwrap().1;
        let mut tokens = body.split_whitespace();
        skip_header(&mut tokens);
        check_matrix(
            &mut tokens,
            &model.input_dense.input_weights,
            42,
            24,
            &format!("{name} input_dense"),
        );
        check_bias(
            &mut tokens,
            &model.input_dense.bias,
            &format!("{name} input_dense"),
        );
        skip_header(&mut tokens);
        check_matrix(
            &mut tokens,
            &model.vad_gru.input_weights,
            24,
            72,
            &format!("{name} vad_gru/in"),
        );
        check_matrix(
            &mut tokens,
            &model.vad_gru.recurrent_weights,
            24,
            72,
            &format!("{name} vad_gru/rec"),
        );
        check_bias(&mut tokens, &model.vad_gru.bias, &format!("{name} vad_gru"));
        skip_header(&mut tokens);
        check_matrix(
            &mut tokens,
            &model.noise_gru.input_weights,
            90,
            144,
            &format!("{name} noise_gru/in"),
        );
        check_matrix(
            &mut tokens,
            &model.noise_gru.recurrent_weights,
            48,
            144,
            &format!("{name} noise_gru/rec"),
        );
        check_bias(
            &mut tokens,
            &model.noise_gru.bias,
            &format!("{name} noise_gru"),
        );
        skip_header(&mut tokens);
        check_matrix(
            &mut tokens,
            &model.denoise_gru.input_weights,
            114,
            288,
            &format!("{name} denoise_gru/in"),
        );
        check_matrix(
            &mut tokens,
            &model.denoise_gru.recurrent_weights,
            96,
            288,
            &format!("{name} denoise_gru/rec"),
        );
        check_bias(
            &mut tokens,
            &model.denoise_gru.bias,
            &format!("{name} denoise_gru"),
        );
        skip_header(&mut tokens);
        check_matrix(
            &mut tokens,
            &model.denoise_output.input_weights,
            96,
            22,
            &format!("{name} denoise_output"),
        );
        check_bias(
            &mut tokens,
            &model.denoise_output.bias,
            &format!("{name} denoise_output"),
        );
        skip_header(&mut tokens);
        check_matrix(
            &mut tokens,
            &model.vad_output.input_weights,
            24,
            1,
            &format!("{name} vad_output"),
        );
        check_bias(
            &mut tokens,
            &model.vad_output.bias,
            &format!("{name} vad_output"),
        );
        assert!(tokens.next().is_none(), "{} has trailing tokens", name);
    }
}

#[test]
fn loaded_states_infer_deterministically_and_differ_from_bundled() {
    nnnoiseless::prepare();
    // Stimulus uses the API's 16-bit PCM-scale unit: production scales
    // normalized audio by 32768 around process_frame, and the upstream
    // demo assigns raw int16 samples as floats. Normalized-range noise
    // would pin every model to identical delayed-input passthrough.
    let mut noise = vec![0.0f32; DenoiseState::FRAME_SIZE];
    let mut state = 0xA17Eu32;
    for sample in noise.iter_mut() {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *sample = (state as f32 / u32::MAX as f32 - 0.5) * 0.6 * 32768.0;
    }
    for (name, bytes) in [("lq", LQ_RNNN), ("sh", SH_RNNN)] {
        let model = parse_rnnn_model(bytes).unwrap();
        let mut first = DenoiseState::from_model(model.clone()).unwrap();
        let mut second = DenoiseState::from_model(model).unwrap();
        let mut bundled = DenoiseState::new();
        let mut out_first = vec![0.0f32; DenoiseState::FRAME_SIZE];
        let mut out_second = vec![0.0f32; DenoiseState::FRAME_SIZE];
        let mut out_bundled = vec![0.0f32; DenoiseState::FRAME_SIZE];
        for _ in 0..4 {
            first.process_frame(&mut out_first, &noise);
            second.process_frame(&mut out_second, &noise);
            bundled.process_frame(&mut out_bundled, &noise);
        }
        assert!(out_first.iter().all(|s| s.is_finite()), "{}", name);
        assert_eq!(out_first, out_second, "{name} is nondeterministic");
        let max_diff = out_first
            .iter()
            .zip(&out_bundled)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        println!("{name}-vs-bundled max frame diff: {max_diff}");
        assert_ne!(out_first, out_bundled, "{name} matches bundled inference");
    }
}
