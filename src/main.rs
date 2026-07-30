mod timer;

use std::fmt::Display;
use std::io::{self, Write};
use std::process::Command;
use std::{collections::HashMap, error::Error, fs::File, io::BufReader, path::Path};

use pbn::{Step, StepProvider, Timer, ValidityChecker};

#[derive(Debug)]
enum AOStep {
    Prune(aograph::AIdx),
    Seq(Vec<AOStep>, String),
}

impl AOStep {
    fn show(&self, ao: &aograph::Graph) -> String {
        match self {
            AOStep::Prune(aidx) => format!("prune {}", ao.and_at(*aidx)),
            AOStep::Seq(aosteps, label) => {
                format!(
                    "{} ({})",
                    label,
                    aosteps
                        .iter()
                        .map(|s| s.show(&ao))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

fn siblings(ao: &aograph::Graph, aidx: aograph::AIdx) -> Vec<aograph::AIdx> {
    ao.providers(ao.conclusion(aidx))
        .filter(|a| *a != aidx)
        .collect()
}

impl pbn::Step for AOStep {
    type Exp = aograph::Graph;

    fn apply(&self, e: &Self::Exp) -> Option<Self::Exp> {
        match self {
            AOStep::Prune(aidx) => {
                let mut res = e.clone();
                if siblings(e, *aidx).is_empty() {
                    return None;
                }
                res.and_remove(*aidx);
                res.remove_disconnected();
                Some(res)
            }
            AOStep::Seq(aosteps, _) => {
                let mut res = e.clone();
                for s in aosteps {
                    res = s.apply(e)?;
                }
                Some(res)
            }
        }
    }
}

fn read_ao(path: &Path) -> Result<aograph::Graph, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let jd: jsongraph::Data = serde_json::from_reader(reader)?;
    let jsongraph::Data::Single { graph } = jd else {
        return Err("Multi not supported")?;
    };
    let ao = aograph::Graph::try_from(graph)?;
    Ok(ao)
}

fn display(path_prefix: &Path, ao: &aograph::Graph) -> Result<(), Box<dyn Error>> {
    let dot_path = path_prefix.with_extension("dot");
    let dot_contents = ao.dot(&HashMap::new());
    let mut dot_file = File::create(&dot_path)?;
    write!(dot_file, "{}", dot_contents)?;

    let pdf_path = path_prefix.with_extension("pdf");
    let pdf_contents = Command::new("dot")
        .arg("-Tpdf")
        .arg("-Nfontname=Linux Biolinum")
        .arg("-Nfontsize=16")
        .arg("-Efontname=Linux Biolinum")
        .arg("-Efontsize=16")
        .arg(dot_path)
        .output()?
        .stdout;
    let mut pdf_file = File::create(pdf_path)?;
    pdf_file.write_all(&pdf_contents)?;

    Ok(())
}

fn children_with_siblings(ao: &aograph::Graph) -> Vec<aograph::AIdx> {
    ao.and_indexes()
        .filter(|aidx| !siblings(&ao, *aidx).is_empty())
        .collect()
}

struct All;

impl<T: Timer> StepProvider<T> for All {
    type Step = AOStep;

    fn provide(
        &mut self,
        _timer: &T,
        e: &<Self::Step as Step>::Exp,
    ) -> Result<Vec<Self::Step>, T::EarlyCutoff> {
        let mut res = vec![];
        for aidx in children_with_siblings(e) {
            res.push(AOStep::Prune(aidx))
        }
        Ok(res)
    }
}

struct Check;

impl ValidityChecker for Check {
    type Exp = aograph::Graph;

    fn check(&mut self, e: &Self::Exp) -> bool {
        children_with_siblings(e).is_empty()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut ao = read_ao(Path::new("examples/moderate.json"))?;
    let timer = timer::Timer::infinite();
    let mut controller = pbn::Controller::new(timer, All, Check, ao, true);

    while !controller.valid() {
        display(Path::new("out/out.dot"), controller.working_expression())?;
        display(Path::new("out/out.dot"), controller.working_expression())?;

        let mut options = controller.provide()?;

        if options.is_empty() {
            println!("Not possible!");
            return Ok(());
        }

        for (i, s) in options.iter().enumerate() {
            println!("  {}) {}", i + 1, s.show(controller.working_expression()));
        }

        let idx = loop {
            print!("Select a step ('q' to quit): ");
            std::io::stdout().flush().unwrap();

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            if input == "q" {
                return Ok(());
            }

            match input.parse::<usize>() {
                Ok(choice) => {
                    if 1 <= choice && choice <= options.len() {
                        break choice - 1;
                    } else {
                        continue;
                    }
                }
                Err(_) => continue,
            };
        };

        controller.decide(options.swap_remove(idx))
    }

    display(Path::new("out/out.dot"), controller.working_expression())?;
    println!("All done!");
    Ok(())
}
