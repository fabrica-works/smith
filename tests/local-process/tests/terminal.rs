use skein_io::kernel;
use skein_lib::{Time, Wall};
use skein_sim::{Config, Event, Sim, Summary};
use skein_world::Host;
use smith_local_process_world::terminal::Terminal;

#[test]
fn a_piped_person_shows_whole_output_in_at_most_sixteen_reads_then_closes() {
    let bytes: Vec<u8> = (0..60_000).map(|index| u8::try_from(index % 251).expect("byte")).collect();
    let mut sim = Sim::new(7, Config { buffer: 60_000, ..Config::calm() });
    let process = sim.spawn_process();
    let input = sim.open_inherited_write(process);
    let output = sim.open_inherited_read(process);
    assert_eq!(sim.peer_feed(process, output, &bytes), bytes.len());
    sim.peer_close(process, output);
    let mut person = Terminal::piped(input, output, vec![]);
    person.finished(kernel::Exit::Code(0));
    for _ in 0..32 {
        person.iterate(Time::ZERO, Wall::EPOCH);
        assert!(!person.is_empty(), "person waits for the output close completion");
        let closing =
            person.submissions().iter().any(|submit| matches!(submit.kind, kernel::Op::Close { fd } if fd == output));
        sim.submit(process, person.submissions());
        sim.reap(process, person.completions());
        if closing {
            person.iterate(Time::ZERO, Wall::EPOCH);
            break;
        }
    }
    assert_eq!(person.shown(), bytes);
    let reads = sim
        .trace()
        .iter()
        .filter(|entry| matches!(entry.event, Event::Submit { kind: Summary::PipeRead { fd, .. }, .. } if fd == output))
        .count();
    assert!(reads <= 16, "whole output and EOF took {reads} pipe reads");
    assert!(person.is_empty());
    sim.assert_quiescent(process);
    sim.assert_no_open_fds(process);
}
