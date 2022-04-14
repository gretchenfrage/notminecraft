use ab_glyph::*;
use blake2::{Blake2s256, Digest};
use criterion::{criterion_group, criterion_main, Criterion};
use std::io::Write;

const OPENS_SANS_ITALIC: &[u8] = include_bytes!("../fonts/OpenSans-Italic.ttf");
const EXO2_TTF: &[u8] = include_bytes!("../fonts/Exo2-Light.ttf");
const EXO2_OTF: &[u8] = include_bytes!("../fonts/Exo2-Light.otf");

const SENTENCE: &str =
    "a set of words that is complete in itself, typically containing a subject and predicate, \
     conveying a statement, question, exclamation, or command, and consisting of a main \
     clause and sometimes one or more subordinate clauses.";

fn bench_layout_a_sentence(c: &mut Criterion) {
    c.bench_function("layout_a_sentence", |b| {
        let font = FontRef::try_from_slice(OPENS_SANS_ITALIC).unwrap();
        let mut glyphs = vec![];

        b.iter(|| {
            glyphs.clear();
            dev::layout_paragraph(
                font.as_scaled(25.0),
                point(100.0, 25.0),
                600.0,
                SENTENCE,
                &mut glyphs,
            );
        });

        // verify the layout result against static reference hash
        let mut hash = Blake2s256::default();
        for g in glyphs {
            write!(
                hash,
                "{id}:{scale_x}:{scale_y}:{pos_x}:{pos_y}",
                id = g.id.0,
                scale_x = g.scale.x,
                scale_y = g.scale.y,
                pos_x = g.position.x,
                pos_y = g.position.y,
            )
            .unwrap();
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "e3ae01bfc47bcbfe9a2a060ef651cf466798410c60652540d467d5332a8fe028"
        );
    });
}

fn bench_layout_a_sentence_vec(c: &mut Criterion) {
    c.bench_function("layout_a_sentence (FontVec::try_from_vec)", |b| {
        let font = FontVec::try_from_vec(OPENS_SANS_ITALIC.to_vec()).unwrap();
        let mut glyphs = vec![];

        b.iter(|| {
            glyphs.clear();
            dev::layout_paragraph(
                font.as_scaled(25.0),
                point(100.0, 25.0),
                600.0,
                SENTENCE,
                &mut glyphs,
            );
        });

        // verify the layout result against static reference hash
        let mut hash = Blake2s256::default();
        for g in glyphs {
            write!(
                hash,
                "{id}:{scale_x}:{scale_y}:{pos_x}:{pos_y}",
                id = g.id.0,
                scale_x = g.scale.x,
                scale_y = g.scale.y,
                pos_x = g.position.x,
                pos_y = g.position.y,
            )
            .unwrap();
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "e3ae01bfc47bcbfe9a2a060ef651cf466798410c60652540d467d5332a8fe028"
        );
    });
}

fn bench_layout_a_sentence_arc_slice(c: &mut Criterion) {
    c.bench_function("layout_a_sentence (FontArc::try_from_slice)", |b| {
        let font = FontArc::try_from_slice(OPENS_SANS_ITALIC).unwrap();
        let mut glyphs = vec![];

        b.iter(|| {
            glyphs.clear();
            dev::layout_paragraph(
                font.as_scaled(25.0),
                point(100.0, 25.0),
                600.0,
                SENTENCE,
                &mut glyphs,
            );
        });

        // verify the layout result against static reference hash
        let mut hash = Blake2s256::default();
        for g in glyphs {
            write!(
                hash,
                "{id}:{scale_x}:{scale_y}:{pos_x}:{pos_y}",
                id = g.id.0,
                scale_x = g.scale.x,
                scale_y = g.scale.y,
                pos_x = g.position.x,
                pos_y = g.position.y,
            )
            .unwrap();
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "e3ae01bfc47bcbfe9a2a060ef651cf466798410c60652540d467d5332a8fe028"
        );
    });
}

fn bench_layout_a_sentence_otf(c: &mut Criterion) {
    c.bench_function("layout_a_sentence (exo2-otf)", |b| {
        let font = FontRef::try_from_slice(EXO2_OTF).unwrap();
        let mut glyphs = vec![];

        b.iter(|| {
            glyphs.clear();
            dev::layout_paragraph(
                font.as_scaled(25.0),
                point(100.0, 25.0),
                600.0,
                SENTENCE,
                &mut glyphs,
            );
        });

        // verify the layout result against static reference hash
        let mut hash = Blake2s256::default();
        for g in glyphs {
            write!(
                hash,
                "{id}:{scale_x}:{scale_y}:{pos_x}:{pos_y}",
                id = g.id.0,
                scale_x = g.scale.x,
                scale_y = g.scale.y,
                pos_x = g.position.x,
                pos_y = g.position.y,
            )
            .unwrap();
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "5c19fee8e6440b3e6fb1c7d0b2a5b3d2354f2f7ccbc2ff4b53ba96cd4e6e37ba"
        );
    });
}

fn bench_layout_a_sentence_ttf(c: &mut Criterion) {
    c.bench_function("layout_a_sentence (exo2-ttf)", |b| {
        let font = FontRef::try_from_slice(EXO2_TTF).unwrap();
        let mut glyphs = vec![];

        b.iter(|| {
            glyphs.clear();
            dev::layout_paragraph(
                font.as_scaled(25.0),
                point(100.0, 25.0),
                600.0,
                SENTENCE,
                &mut glyphs,
            );
        });

        // verify the layout result against static reference hash
        let mut hash = Blake2s256::default();
        for g in glyphs {
            write!(
                hash,
                "{id}:{scale_x}:{scale_y}:{pos_x}:{pos_y}",
                id = g.id.0,
                scale_x = g.scale.x,
                scale_y = g.scale.y,
                pos_x = g.position.x,
                pos_y = g.position.y,
            )
            .unwrap();
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "5c19fee8e6440b3e6fb1c7d0b2a5b3d2354f2f7ccbc2ff4b53ba96cd4e6e37ba"
        );
    });
}

criterion_group!(
    name = layout_benches;
    config = Criterion::default().sample_size(400);
    targets = bench_layout_a_sentence,
        bench_layout_a_sentence_vec,
        bench_layout_a_sentence_arc_slice,
        bench_layout_a_sentence_otf,
        bench_layout_a_sentence_ttf,
);

criterion_main!(layout_benches);
