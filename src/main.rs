use std::io::{self, Write};
use std::process::Command;
use std::{collections::HashMap, error::Error, fs::File, io::BufReader, path::Path};

use pbn::Step;

enum AOStep {
    Prune(aograph::AIdx),
    Seq(Vec<AOStep>, String),
}

impl pbn::Step for AOStep {
    type Exp = aograph::Graph;

    fn apply(&self, e: &Self::Exp) -> Option<Self::Exp> {
        match self {
            AOStep::Prune(aidx) => {
                let mut res = e.clone();
                let siblings = e.providers(e.conclusion(*aidx)).collect::<Vec<_>>();
                if siblings.len() == 1 {
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

fn parse_single_step(ao: &aograph::Graph, input: &str) -> Option<AOStep> {
    let aidx = ao.find_and_by_id(input)?;
    Some(AOStep::Prune(aidx))
}

fn parse_step(ao: &aograph::Graph, input: &str) -> Option<AOStep> {
    if input.contains(";") {
        let mut steps = vec![];
        for part in input.split(";") {
            steps.push(parse_single_step(ao, part)?);
        }
        Some(AOStep::Seq(steps, "".to_owned()))
    } else {
        parse_single_step(ao, input)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut ao = read_ao(Path::new("examples/moderate.json"))?;
    loop {
        display(Path::new("out/out.dot"), &ao)?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        let Some(step) = parse_step(&ao, &input) else {
            println!("Cannot parse step");
            continue;
        };

        let Some(new_ao) = step.apply(&ao) else {
            println!("Cannot apply step");
            continue;
        };

        ao = new_ao;
    }
}
