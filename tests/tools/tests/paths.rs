//! Paths at their written and resolved byte bounds in the tools world.

use skein_lib::Time;
use smith_domain_tools::{Cut, Grants, Limits, Next, Outcome, Part, Path};
use smith_tools_world::calls::read;
use smith_tools_world::fixture::Fixture;
use smith_tools_world::{Settings, Step, World};

const PATH_BOUND: u32 = 32;
const ITERATIONS: u32 = 100_000;
const INSPECT: Grants = Grants { inspect: true, modify: false, shell: false };

fn answer(path: &[u8]) -> Outcome {
    let mut fixture = Fixture::new();
    fixture.checkout.write(b"work/temper/src/lib.rs", b"at the bound\n");
    fixture.checkout.write(b"work/temper/xxxxxxxxxxxxxxxxxxxx", b"at the bound\n");
    let authority = fixture.authority(INSPECT);
    let calm = Settings::calm(9601);
    let settings = Settings { tools: Limits { path_bytes: PATH_BOUND, ..calm.tools }, ..calm };
    let mut world = World::new(settings, fixture.checkout);
    let session = world.session(Time::ZERO, authority, vec![Step::Calls(vec![read(path)])]);
    world.run(ITERATIONS);
    assert_eq!(world.answers(session).len(), 1);
    world.answers(session)[0].clone()
}

#[test]
fn a_path_at_its_bound_and_one_past_as_written_and_as_resolved() {
    for path in [
        b"./././././././././././src/lib.rs".as_slice(),
        b".//.//.//.//.//.//.//.//.//.//.//src//lib.rs///",
        b"src/../src/../././././src/lib.rs",
        b"xxxxxxxxxxxxxxxxxxxx",
        b"/work//temper//xxxxxxxxxxxxxxxxxxxx///",
    ] {
        assert_eq!(
            answer(path),
            Outcome::Read {
                content: b"at the bound\n".as_slice().into(),
                first: 1,
                lines: 1,
                total: 1,
                cut: Cut::Whole,
                next: Next::End,
                window: smith_domain_tools::BytesClamp::Within,
            },
            "{path:?}",
        );
    }
}

#[test]
fn a_path_one_byte_past_its_bound_is_refused() {
    for path in [
        b"./././././././././././src/main.rs".as_slice(),
        b".//.//.//.//.//.//.//.//.//.//.//src//main.rs///",
        b"xxxxxxxxxxxxxxxxxxxxx",
        b"/work//temper//xxxxxxxxxxxxxxxxxxxxx///",
    ] {
        assert_eq!(answer(path), Outcome::TooLong, "{path:?}");
    }
}

#[test]
fn the_world_decodes_an_empty_path_as_current_directory() {
    assert_eq!(smith_tools_world::translate::path(b""), Path { absolute: false, parts: Box::new([Part::Current]) });
}
