use std::{cmp::Ordering, iter, thread, time::Duration, env};

use eyre::{Report, Result};
use rand::{thread_rng, Rng};

fn main() -> Result<()> {
    // simulation parameters

    let args: Vec<_> = env::args().collect();
    let args = Args::new(args.iter().skip(1).map(|a| a.as_str()))?;

    if !args.start {
        return Ok(());
    }

    // create simulation
    let mut sim = AntRod::from_args(&args);

    let sleep = Duration::from_millis(args.sleep);

    // run the simulation
    sim.draw();
    while sim.has_ants() {
        thread::sleep(sleep);
        sim.step();
        sim.draw();
    }

    Ok(())
}

// the simulation structure
struct AntRod {
    // the vector is always ordered by position
    ants: Vec<Ant>,
    ant_step: f32,
    drawer: Drawer,
    time: usize,
}

impl AntRod {
    /// Creates new simulation of ant rod,
    /// Resolution is the resolution of the drawn output, ant_step is how much
    /// the ants step with each simulatino step
    fn from_args(args: &Args) -> Self {
        // create vector of ants on the rod
        let mut ants = Vec::new();
        ants.reserve(args.ant_count);
        ants.extend(iter::from_fn(|| Some(Ant::default())).take(args.ant_count));

        // random positions

        if args.regular {
            // regular spacing with ants facing the furtherer side and molly in
            // center
            let dis = 1. / (args.ant_count as f32 + 1.);

            // ants on the left
            for i in 0..(args.ant_count / 2) {
                ants[i] = Ant {
                    position: dis * i as f32 + dis,
                    speed: 1.,
                    typ: AntType::Some,
                };
            }

            // molly
            ants[args.ant_count / 2] = Ant {
                position: 0.5,
                speed: 1.,
                typ: AntType::Molly,
            };

            // ants on the right
            for i in (args.ant_count / 2 + 1)..args.ant_count {
                ants[i] = Ant {
                    position: dis * i as f32 + dis,
                    speed: -1.,
                    typ: AntType::Some,
                };
            }
        } else {
            // random positions
            ants.sort_by(|a, b| {
                a.position
                    .partial_cmp(&b.position)
                    .unwrap_or(Ordering::Equal)
            });
            ants[args.molly_index].typ = AntType::Molly;
        }

        Self {
            ants,
            ant_step: args.ant_step,
            drawer: Drawer::new(args.resolution),
            time: 0,
        }
    }

    fn step(&mut self) {
        // update positions
        for a in &mut self.ants {
            a.position += a.speed * self.ant_step;
        }

        // sort by position, but retain types
        let typ: Vec<_> = self.ants.iter().map(|a| a.typ).collect();
        self.ants.sort_by(|a, b| {
            a.position
                .partial_cmp(&b.position)
                .unwrap_or(Ordering::Equal)
        });
        for (a, t) in self.ants.iter_mut().zip(typ.iter()) {
            a.typ = *t;
        }

        // remove those that have fallen

        // remove from the end
        while self
            .ants
            .last()
            .map(|a| a.position >= 1.)
            .unwrap_or_default()
        {
            self.ants.pop();
        }

        // remove from the front
        self.ants.drain(
            0..self
                .ants
                .iter()
                .position(|a| a.position >= 0.)
                .unwrap_or(self.ants.len()),
        );

        self.time += 1;
    }

    fn has_ants(&self) -> bool {
        !self.ants.is_empty()
    }

    fn draw(&mut self) {
        self.drawer
            .draw(&self.ants, self.time as f32 * self.ant_step);
    }
}

#[derive(Clone)]
struct Ant {
    position: f32,
    // the speed of the ant
    speed: f32,
    typ: AntType,
}

impl Default for Ant {
    fn default() -> Self {
        Self {
            position: thread_rng().gen_range(0.0..1.),
            speed: if thread_rng().gen_bool(0.5) { 1. } else { -1. },
            typ: AntType::Some,
        }
    }
}

#[derive(Clone, PartialEq, Default, Copy)]
enum AntType {
    #[default]
    None,
    Some,
    Molly,
}

impl AntType {
    fn set(&mut self, other: AntType) {
        match (&self, other) {
            (AntType::Molly, AntType::Some) => {}
            (_, a) => *self = a,
        }
    }
}

impl ToString for AntType {
    fn to_string(&self) -> String {
        // white space
        const NO_ANT: &str = "\x1b[47m ";
        // black on white
        const AN_ANT: &str = "\x1b[30m\x1b[47m●";
        // magenta on white
        const MOLLY: &str = "\x1b[35m\x1b[47m●";

        let ant = match self {
            AntType::None => NO_ANT,
            AntType::Some => AN_ANT,
            AntType::Molly => MOLLY,
        };

        format!("{ant}\x1b[0m")
    }
}

struct Drawer {
    ant_vec: Vec<AntType>,
    buffer: String,
}

impl Drawer {
    fn new(resolution: usize) -> Self {
        Self {
            ant_vec: vec![AntType::None; resolution],
            buffer: String::new(),
        }
    }

    /// Expects that `ants` is ordered by position
    fn draw(&mut self, ants: &Vec<Ant>, time: f32) {
        self.ant_vec.fill(AntType::None);

        // set the ants to their positions
        for a in ants {
            let pos = (a.position * self.ant_vec.len() as f32) as usize;
            self.ant_vec[pos].set(a.typ)
        }

        self.buffer.clear();
        // move 2 lines up and left, clear all from cursor to the end
        self.buffer += "\x1b[2F\x1b[0J";
        for a in &self.ant_vec {
            self.buffer += &a.to_string();
        }

        println!("{}\ntime: {:.1}s", self.buffer, time * 100.0);
    }
}

// simulation parameters
struct Args {
    ant_count: usize,
    molly_index: usize,
    ant_step: f32,
    sleep: u64,
    regular: bool,
    resolution: usize,
    start: bool,
}

impl Args {
    fn new<'a>(mut args: impl Iterator<Item = &'a str>) -> Result<Self> {
        macro_rules! next {
            ($t:ty, $i:ident, $n:expr) => {
                match $i.next() {
                    Some(a) => a.parse::<$t>()?,
                    None => {
                        return Err(Report::msg(format!(
                            "missing argument after {}",
                            $n
                        )))
                    }
                }
            };
        }

        let mut res = Args {
            ant_count: 25,
            molly_index: usize::MAX,
            ant_step: 0.001,
            sleep: 50,
            regular: false,
            resolution: terminal_size::terminal_size()
                .unwrap_or((
                    terminal_size::Width(100),
                    terminal_size::Height(100),
                ))
                .0
                 .0
                .into(),
            start: true,
        };

        while let Some(a) = args.next() {
            match a {
                "-c" | "--count" => res.ant_count = next!(usize, args, a),
                "-m" | "--molly" => res.molly_index = next!(usize, args, a),
                "-s" | "--speed" => res.ant_step = next!(f32, args, a),
                "-d" | "--delta" => res.sleep = next!(u64, args, a),
                "--regular" => res.regular = true,
                "-r" | "--resolution" => {
                    res.resolution = next!(usize, args, a)
                }
                "-h" | "-?" | "-help" | "--help" => {
                    help();
                    res.start = false;
                },
                _ => return Err(Report::msg(format!("invalid argument {a}"))),
            }
        }

        if res.molly_index == usize::MAX {
            res.molly_index = res.ant_count / 2;
        }

        if res.molly_index >= res.ant_count {
            return Err(Report::msg(format!(
                "Invalid molly index {} out of {}",
                res.molly_index, res.ant_count
            )));
        }

        Ok(res)
    }
}

fn help() {
    println!(
        "Welcome in {g}{i}stick_ants{r} by {}{}{}

{g}Usage:{r}
  {w}stick_ants{r} {d}[<flags>]{r}
    runs the simulation

{g}Flags:{r}
  {y}-h  -?  -help  --help{r}
    shows this help

  {y}-c  --count{r} {w}<template name>{r}
    sets the total amount of ants (default is 25)

  {y}-m  --molly {w}<molly index>{r}
    creates new template from the directory with the name (center is default)

  {y}-s --speed{r} {w}<speed>{r}
    how fast the simulation runs (default is 0.001)

  {y}-d  --delta{r} {w}<delta time>{r}
    sets the time to wait between each step in milliseconds (default is 50)

  {y}--regular{r}
    enables special case

  {y}-r --resolution{r}
    how many characters should be used for the simulation
",
        // BonnyAD9 gradient in 3 strings
        "\x1b[38;2;250;50;170mB\x1b[38;2;240;50;180mo\x1b[38;2;230;50;190mn",
        "\x1b[38;2;220;50;200mn\x1b[38;2;210;50;210my\x1b[38;2;200;50;220mA",
        "\x1b[38;2;190;50;230mD\x1b[38;2;180;50;240m9\x1b[0m",
        g = "\x1b[92m", // green
        i = "\x1b[23m", // italic
        r = "\x1b[0m",  // reset
        w = "\x1b[97m", // white
        d = "\x1b[90m", // dark gray
        y = "\x1b[93m"  // yellow
    );
}
