# Session 02: the shell's loop, and worlds that end as shipped

Provisional, 2026-10-12. Only terminal modes are left: raw input at
startup, its restoration at the end, and the window's size, as shell
effects of skein's kit (skein `shell.md`, 6.3; `io.md`, 7; `kernel.md`,
6.3). An interactive front end needs them to read a key at a time and to
draw at the terminal's width; without them it runs in line mode. skein
builds a part when its first user pulls it: the first user is smith's
interactive front end, whose raw input (smith 10.10) waits for this
session.

## Read first

- skein `docs/design/shell.md`, sections 6, 6.3, 9, 10 and 13;
  `io.md`, sections 3 and 7; `kernel.md`, sections 6.3, 7, 8 and 10;
  `simulator.md`, section 3.
- `docs/foundation/programming-model.md`, section 2.1;
  `testing-strategy.md`, sections 2.9 and 6 (the teardown invariant).
- decisions.md, 6.2.

## What the design asks

- **Raw input at startup** (shell.md, 6.3): "If standard input is a
  terminal and the configuration asks for it, startup reads the
  terminal's modes, keeps them, and sets the ones asked for: no echo, no
  line editing, and the terminal's own signals kept or not, as the
  service says." Standard input is then adopted as a read stream;
  otherwise "the service hears that it has none".
- **Restored at the end:** "`main` restores the kept modes once `drive`
  returns, on every path out of it, before it writes its last line to
  standard error."
- **The window's size:** read at startup; `SIGWINCH` "blocked with the
  termination signals", read from the same signalfd, and told as
  `Resized`; then `Window { owner, stream }`, answered `Window { owner,
  size }`, or `Gone` once the terminal hung up (io.md, 7). The record is
  synchronous, `TIOCGWINSZ` (kernel.md, 6.3), and comes with its
  conformance scenario on both backends (kernel.md, 10).
- **Tested end to end under a pseudo-terminal** (shell.md, 9): "input
  read a key at a time, the modes found again once the binary exits, and
  a size change heard".

## Choices this plan makes

- **One signalfd, two meanings never mixed:** `Done::ServiceSignal`
  becomes `Done::Signalled(Signalled)`; io tells a termination as
  `Shutdown` and `SIGWINCH` as `Resized`, so `Shutdown`'s
  `ServiceSignal` never holds a resize.
- **`Kept` is a value `main` holds across `drive`,** not shell state. A
  service with no terminal calls neither `raw_input` nor
  `open_signals`, and `open_termination_signals` stays for it.
- **The kit gives tests the other end:** `resize_terminal` sets a
  pseudo-terminal's size from its master, and `terminal_modes` reads the
  modes through it, so a scripted person can make and see both. The
  terminal syscalls live only in the ring adapter's module.
- **A fixture binary:** `tests/world` gains `world_terminal_fixture`, a
  service of a few dozen lines on `drive` that sets raw input, reports
  each key and each size on standard error, and restores at exit.
- **Start:** at once; it needs nothing.

## Rules that bind it

- **skein never names smith's code** (development.md, 3.1): before
  `land`, `git diff origin/main -- crates testing tests | rg -c '^\+.*smith'`
  prints nothing; documents may name smith as the feature's reason.
- **The only `unsafe` a service runs is the ring adapter's** (shell.md,
  1; `skein-shell/src/lib.rs`): `tcgetattr`, `tcsetattr`, `TIOCGWINSZ`
  and `TIOCSWINSZ` live in the ring adapter (`crates/skein-shell/src/ring.rs`
  and `src/ring/`), each call with its safety comment.
- **No world opts out of the teardown invariant**
  (testing-strategy.md, 6): the fixture's worlds pass it.

## Landed in the first run

| Increment | Commit | Proof |
|---|---|---|
| 02.1 `Host` and `drive` in skein-shell | `220e138c` | deterministic |
| 02.2 a termination signal in simulated worlds | `50c8c205` | deterministic |
| 02.3 the echo drains at a shutdown | `1f750dd2` | deterministic |
| 02.4 the teardown invariant | `5f23a22b` | merged, unproven: `smoke-tools`' teardown check |
| 02.5 shipped times, fakes that never hang up | `ec0f4081` | deterministic |
| 02.bridge-writer-stderr: hosted writers to their stderr pipes | `55ac10d6` | deterministic |
| 02.7 the fake TLS transport reads every byte | `198dedf6` | deterministic |
| 02.8 the process ID at startup | `6cd32111` | deterministic |

## Increments

### 02.6 Terminal modes
Scope (shell.md, 6.3; io.md, 7; kernel.md, 6.3 and 10): startup's raw
input with its kept modes and its size, or no terminal; the modes
restored by `main`; `SIGWINCH` blocked with the termination signals and
told as `Resized`; the synchronous `Window` record, its io request and
terminal; the simulator's answer to `Window`; the kit's
`resize_terminal` and `terminal_modes` for tests.
Needs: nothing, from smith or elsewhere.
Files: `crates/skein-io` (`src/kernel.rs` `Op`, `Done`, `Shape`;
`src/records.rs`; `src/signals.rs`; `src/layer.rs`, `Window` beside
`Request::Usage` at :516); users of `Done::ServiceSignal`:
`skein-io` (`file_layer/store.rs`, `listener.rs`, `pipe.rs`,
`process.rs`, `stream.rs`, `tests/kernel.rs`, `tests/signals.rs`),
`crates/skein-shell/src/ring.rs` (:939), `crates/skein-sim`
(`src/sim.rs`, `src/sim/processes.rs`), `testing/skein-conformance`
(`src/run.rs`, `src/scenarios.rs`), `testing/skein-world/src/real.rs`,
`tests/sim/tests/processes.rs`, `tests/world` (`src/hosted.rs`,
`src/lib.rs`, `tests/drive.rs`); `crates/skein-shell` (`src/ring.rs`,
`src/ring/process.rs:62`, new `src/terminal.rs`, `src/lib.rs`);
`tests/conformance/{ring,sim}/tests/conformance.rs`; `tests/world`
(`Cargo.toml`, new `src/bin/terminal_fixture.rs`, `src/hosted.rs`'s
`Act`, `tests/end_to_end.rs`). Shared with no session of this plan.
Shape: `kernel::{Op::Window, Done::Signalled, Done::Window, Size,
Signalled}`; `records::{Request::Window, Event::Resized, Event::Window,
Window}`; `skein_shell::{Keys, Input, Kept, Watch, Modes, raw_input,
open_signals, resize_terminal, terminal_modes}` (sketch).
Test shape:
- Step, in `crates/skein-io/src/tests/signals.rs`:
  `a_window_change_is_told_as_resized_and_never_as_shutdown`,
  `a_window_request_has_one_terminal`,
  `a_window_on_a_terminal_that_hung_up_is_gone`.
- Conformance, written as `usage_counts_the_child_only_after_reaping`
  (`tests/conformance/ring/tests/conformance.rs:224`, sim's :278):
  `window_on_a_descriptor_that_is_not_a_terminal_answers_other`, on both
  backends.
- End to end in `tests/world/tests/end_to_end.rs`, through `run` (:108)
  with `Mode::Terminal`, the fixture beside `world_process_fixture`
  (`tests/world/Cargo.toml:39`), and `Act::Resize` and `Act::Modes`
  added to `Act` (`tests/world/src/hosted.rs:10`):
  ```rust
  #[test]
  fn input_is_read_a_key_at_a_time() {
      let outcome = run(terminal_fixture(), Mode::Terminal, &[Act::Write(0, b"aq"), Act::Read(0), Act::Read(0)], false);
      let (child, _) = observations(&outcome);
      assert_eq!(child.stderr(), b"ready\nkey a\nkey q\n");
      assert_eq!(child.exit_status(), Some(Exit::Code(0)));
  }
  ```
  Also `the_modes_are_found_again_once_the_binary_exits` (`Act::Modes`
  after the end: echo and lines on), `a_size_change_is_heard`
  (`Act::Resize` to 30 rows and 100 columns; `size 30x100` on standard
  error).
- Ceilings: step 20 ms; conformance 0.5 s; end to end 1 s each, the
  world's run bound.
Proof: process, `the_modes_are_found_again_once_the_binary_exits`: a
binary that set raw input leaves its terminal with echo and line
editing on; once, under 1 s.
Don't: no terminal in the simulator; no restore inside `drive`; no
resize while a test does not ask for one.
Size: at most 350.
Removes: `Done::ServiceSignal` and `Shape::ServiceSignal`, replaced by
`Signalled`.

## Interface sketches

A window change and a termination arrive on one signalfd and are never
one type. The owner of the stream that reads the terminal asks `Window`
after `Resized`; `main` restores `Kept` once `drive` returns.

```rust
// crates/skein-io/src/kernel.rs
pub enum Op {
    // exists on main, and:
    /// The size of the terminal open on `fd` (`TIOCGWINSZ`), synchronous; `Other` for a descriptor that is not one, or hung up.
    Window { fd: Fd },
}
pub enum Done {
    // exists on main, `ServiceSignal(ServiceSignal)` replaced by:
    /// What one `ReadSignal` read.
    Signalled(Signalled),
    /// A `Window`'s size.
    Window(Size),
}
/// A terminal's size in character cells.
pub struct Size { pub rows: u16, pub columns: u16 }
/// What one `ReadSignal` read from the service's signalfd.
pub enum Signalled {
    /// `SIGINT` or `SIGTERM`: io tells `Shutdown`.
    Termination(ServiceSignal),
    /// `SIGWINCH`: io tells `Resized`.
    WindowChanged,
}

// crates/skein-io/src/records.rs
pub enum Request {
    // exists on main, and:
    /// The size of the terminal `stream` reads; its one terminal is `Event::Window`.
    Window { owner: Token, stream: Token },
}
pub enum Event {
    // exists on main, and:
    /// The window changed; names no owner, as `Shutdown` does.
    Resized,
    /// A `Window` request's one terminal.
    Window { owner: Token, window: Window },
}
/// What a `Window` request read: the size, or `Gone` once the terminal hung up.
pub enum Window { Size(Size), Gone }

// crates/skein-shell/src/terminal.rs (its syscalls in the ring adapter)
/// Whether the terminal still makes signals of its interrupt keys once raw.
pub enum Keys { Signal, Pass }
/// What startup found on standard input.
pub enum Input { Terminal { kept: Kept, size: Size }, NotATerminal }
/// A terminal's modes as startup found them.
pub struct Kept { /* the termios startup read */ }
/// Which signals startup blocks and reads from the one signalfd.
pub enum Watch { Termination, TerminationAndWindow }
/// The modes a terminal shows, for a test at its other end.
pub struct Modes { pub echo: bool, pub lines: bool, pub signals: bool }
pub fn raw_input(input: Fd, keys: Keys) -> Result<Input, kernel::Error>;
pub fn open_signals(watch: Watch) -> Result<Fd, i32>;
impl Kept { pub fn restore(self) -> Result<(), kernel::Error>; }
pub fn resize_terminal(master: Fd, size: Size) -> Result<(), kernel::Error>;
pub fn terminal_modes(fd: Fd) -> Result<Modes, kernel::Error>;
```

## Questions to raise when reached

- **The simulator and `Window`:** `simulator.md` does not say what it
  answers. Proposal: no simulated terminal, so `Window` answers `Other`,
  as the ring does for a descriptor that is not a terminal; the
  conformance scenario holds both to it.
- **A terminal suspended and resumed** stays open (shell.md, 10): restore
  only at the end.

## Done when

- `SIGWINCH` is watched: `rg -n 'SIGWINCH' crates/skein-shell/src`
  prints a line.
- The three end-to-end stories pass: `heavy cargo nextest run -p
  skein-world-tests --test end_to_end -E
  'test(/key_at_a_time|modes_are_found_again|size_change/)'`.
- The scenario passes on both backends: `heavy cargo nextest run -p
  skein-conformance-ring -p skein-conformance-sim -E
  'test(window_on_a_descriptor)'`.
- `Done::ServiceSignal` is gone: `rg -n 'Done::ServiceSignal' crates
  testing tests` prints nothing.
