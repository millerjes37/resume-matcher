use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType, SentenceEmbeddingsModel};
use rust_bert::pipelines::ner::NERModel;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::collections::HashMap;
use chrono::Local;
use regex::Regex;

mod utils;
mod graph_scoring;
mod scoring;
mod tfidf;

use utils::compute_cosine_similarity;
use graph_scoring::build_knowledge_graph;
use scoring::score_lines;
use tfidf::TfIdfBuilder;

#[derive(Serialize, Deserialize)]
struct ResumeLine {
    job: String,
    line: String,
}

#[derive(Serialize, Deserialize)]
struct ResumeData {
    lines: Vec<ResumeLine>,
    skills: Vec<String>,
}

#[derive(Clone)]
pub struct ResumeLineData {
    line: String,
    job: String,
    embedding: Vec<f32>,
    entities: Vec<(String, Vec<f32>)>,
}

fn main() -> anyhow::Result<()> {
    println!("Initializing models...");
    
    // Initialize models
    let bert_model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .create_model()?;
    let ner_model = NERModel::new(Default::default())?;

    println!("Loading resume data...");
    
    // Load resume data
    let resume_json = fs::read_to_string("resume-lines.json")?;
    let resume_data: ResumeData = serde_json::from_str(&resume_json)?;

    println!("Precomputing embeddings and entities for resume lines...");
    
    // Precompute embeddings and entities for resume lines
    let mut resume_lines_data = Vec::new();
    for line in &resume_data.lines {
        let embedding = bert_model.encode(&[&line.line])?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to get embedding for line: {}", line.line))?;
            
        let entities_res = ner_model.predict(&[&line.line]);
        let entities: Result<Vec<(String, Vec<f32>)>, anyhow::Error> = entities_res
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get NER results"))?
            .iter()
            .map(|entity| {
                let entity_text = entity.word.clone();
                let entity_embedding = bert_model.encode(&[&entity_text])?
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get embedding for entity: {}", entity_text))?;
                Ok((entity_text, entity_embedding))
            })
            .collect();
            
        resume_lines_data.push(ResumeLineData {
            line: line.line.clone(),
            job: line.job.clone(),
            embedding,
            entities: entities?,
        });
    }

    // Ensure output directory exists
    fs::create_dir_all("output")?;

    println!("Processing job descriptions...");
    
    // Process each job description
    for entry in fs::read_dir("job_descriptions")? {
        let path = entry?.path();
        if path.extension().map_or(false, |ext| ext == "txt" || ext == "md") {
            println!("Processing: {:?}", path);
            process_job_description(&path, &resume_lines_data, &resume_data.skills, &bert_model, &ner_model)?;
        }
    }

    println!("Resume generation complete!");
    Ok(())
}

fn process_job_description(
    path: &Path,
    resume_lines: &[ResumeLineData],
    skills: &[String],
    bert_model: &SentenceEmbeddingsModel,
    ner_model: &NERModel,
) -> anyhow::Result<()> {
    const MINIMUM_RELEVANCE_SCORE: f32 = 0.2;

    // Extract company and role from filename (e.g., "AcmeCorp-SoftwareEngineer.txt")
    let filename = path.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename: {:?}", path))?;
    
    let parts: Vec<&str> = filename.split('-').collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!("Filename must be in format 'Company-Role.txt': {}", filename));
    }
    
    let company = parts[0].to_string();
    let role = parts[1].to_string();

    // Read job description
    let jd_text = fs::read_to_string(path)?;
    let sentences: Vec<&str> = jd_text
        .split('.')
        .filter(|s| !s.trim().is_empty())
        .collect();

    // Compute sentence embeddings
    let jd_sentence_embeddings = bert_model.encode(&sentences)?;

    // Build TF-IDF model with sentences as documents
    let mut tfidf_builder = TfIdfBuilder::new();
    for sentence in &sentences {
        tfidf_builder.add(sentence);
    }
    let tfidf_model = tfidf_builder.build();

    // Build knowledge graph (entities with embeddings)
    let jd_entities = build_knowledge_graph(&jd_text, bert_model, ner_model)?;

    // Score resume lines
    let scored_lines = score_lines(resume_lines, &jd_sentence_embeddings, &tfidf_model, &jd_entities, &jd_text);

    // Determine relevant skills (Moved here to be available for highlighting)
    let common_tech_skills: Vec<String> = vec![
        "rust", "python", "java", "c++", "javascript", "aws", "azure", "gcp", "docker", "kubernetes", 
        "sql", "nosql", "linux", "machine learning", "nlp", "data analysis", "react", "angular", "vue",
        "systems architecture", "policy analysis", "legislative monitoring", "legal reasoning", 
        "project management", "business management", "nix", "lobbying" // Adding some from existing master list
    ].into_iter().map(String::from).collect();

    let mut preliminary_skills: Vec<String> = Vec::new();
    let jd_text_lower = jd_text.to_lowercase(); // jd_text is already defined

    // Extract skills from JD entities
    for (entity_text, _) in &jd_entities { // jd_entities is already defined
        let entity_lower = entity_text.to_lowercase();
        if common_tech_skills.contains(&entity_lower) {
            preliminary_skills.push(entity_text.clone()); 
        }
    }

    // Extract skills from candidate's master list if present in JD
    // 'skills' here refers to the function parameter resume_data.skills (master list)
    for skill_master in skills { // 'skills' is a function parameter
        let skill_master_lower = skill_master.to_lowercase();
        if jd_text_lower.contains(&skill_master_lower) {
            preliminary_skills.push(skill_master.clone()); 
        }
    }

    // Deduplicate relevant_skills, preserving order of first appearance
    let mut relevant_skills: Vec<String> = Vec::new(); 
    let mut seen_skills_lower: std::collections::HashSet<String> = std::collections::HashSet::new();
    for skill_prelim in preliminary_skills { 
        if seen_skills_lower.insert(skill_prelim.to_lowercase()) {
            relevant_skills.push(skill_prelim);
        }
    }
    // Now `relevant_skills` is available for both highlighting and the main skills section.

    // Group by job and select top 4 lines
    let mut lines_by_job: HashMap<String, Vec<(String, f32)>> = HashMap::new();
    for (line, score) in scored_lines {
        let job = resume_lines
            .iter()
            .find(|rl| rl.line == line)
            .map(|rl| rl.job.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        lines_by_job.entry(job).or_default().push((line, score));
    }

    let mut selected_lines: HashMap<String, Vec<String>> = HashMap::new();
    for (job, mut lines) in lines_by_job {
        lines.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        let top_lines: Vec<String> = lines
            .into_iter()
            .filter(|(_, score)| *score >= MINIMUM_RELEVANCE_SCORE) // Filter by score
            .take(4)                                              // Then take top 4 of the *remaining*
            .map(|(line, _)| {
                let mut formatted_line = line.clone();
                // `relevant_skills` is in scope here as it was moved before this block.
                for skill_to_highlight in &relevant_skills { 
                    let pattern = format!(r"(?i)\b({})\b", regex::escape(skill_to_highlight));
                    // Handle potential regex compilation error more gracefully if this were production code
                    if let Ok(re) = Regex::new(&pattern) {
                        formatted_line = re.replace_all(&formatted_line, |caps: &regex::Captures| {
                            format!("#strong[{}]", &caps[0])
                        }).into_owned();
                    }
                }
                formatted_line
            })
            .collect();
        
        if !top_lines.is_empty() { // Only insert if there are lines meeting the threshold
            selected_lines.insert(job, top_lines);
        }
    }

    // Generate Typst resume
    let typst_content = generate_typst_resume(&selected_lines, &relevant_skills); // This `relevant_skills` is now correctly defined above
    let date = Local::now().format("%Y-%m-%d").to_string();
    let output_path = format!("output/{}-{}-{}.typ", company, role, date);
    let mut file = File::create(&output_path)?;
    file.write_all(typst_content.as_bytes())?;

    println!("Generated resume: {}", output_path);
    Ok(())
}

fn generate_typst_resume(selected_lines: &HashMap<String, Vec<String>>, relevant_skills: &Vec<String>) -> String {
    let format_lines = |job_key: &str| -> String {
        selected_lines
            .get(job_key)
            .unwrap_or(&vec![])
            .iter()
            .map(|line| format!("- {}", line))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r##"#import "@preview/basic-resume:0.2.8": *

#let name = "Jackson Miller"
#let location = "Wabash, IN"
#let email = "jackson@civitas.ltd"
#let github = "github.com/millerjes37"
#let linkedin = "linkedin.com/in/jackson-e-miller"
#let phone = "+1 (260) 377-9575"
#let personal-site = "jackson-miller.us"

#show: resume.with(
  author: name,
  email: email,
  github: github,
  linkedin: linkedin,
  phone: phone,
  personal-site: personal-site,
  accent-color: "#2d4736",
  font: "Galahad",
  paper: "us-letter",
  author-position: left,
  personal-info-position: left,
)

== Education
#edu(
  institution: "Wabash College",
  location: "Crawfordsville, IN",
  dates: dates-helper(start-date: "Aug 2019", end-date: "May 2023"),
  degree: "Bachelor's of Arts, Political Science and Psychology",
)
- Cumulative GPA: 3.2/4.0 | Dean's List, President's Merit Scholarship, National Merit Scholarship
- Relevant Coursework: Statistics, Program Development, Constitutional Law, American Politics, Debate, Discrete Mathematics, Principles and Practice of Comp Sci, Data Structures, Logical Reasoning

== Ventures
#project(
  role: "Co-owner & CEO",
  name: "Civitas LLC",
  dates: dates-helper(start-date: "May 2024", end-date: "Present"),
  url: "blog.civitas.ltd",
)
{}

#project(
  role: "Managing Member",
  name: "Miller Staking",
  dates: dates-helper(start-date: "April 2023", end-date: "Present"),
)
{}

== Work Experience
#work(
  title: "Marketing Manager",
  location: "Indianapolis, IN",
  company: "Kroger Gardis and Regas LLC",
  dates: dates-helper(start-date: "May 2023", end-date: "September 2023"),
)
{}

#work(
  title: "Education Team Summer Law Clerk",
  location: "Indianapolis, IN",
  company: "Kroger Gardis and Regas LLC",
  dates: dates-helper(start-date: "May 2023", end-date: "September 2023"),
)
{}

#work(
  title: "Advancement Intern",
  location: "Crawfordsville, IN",
  company: "Wabash College",
  dates: dates-helper(start-date: "September 2022", end-date: "May 2023"),
)
{}

#work(
  title: "Substitute Teacher",
  location: "Indiana",
  company: "Various Indiana Public Schools",
  dates: dates-helper(start-date: "September 2022", end-date: "May 2023"),
)
{}

== Professional Development
#work(
  title: "Advisor, Vice President, Recruitment Chair, Risk Manager (non-concurrent)",
  location: "Crawfordsville, IN",
  company: "Phi Delta Theta, IN Beta",
  dates: dates-helper(start-date: "November 2019", end-date: "November 2022"),
)
{}

#work(
  title: "Webmaster",
  location: "Converse, IN",
  company: "Miami County Agricultural Association",
  dates: dates-helper(start-date: "November 2017", end-date: "Present"),
)
{}

== Skills
#set align(center)
{}
"##,
        format_lines("Civitas LLC"),
        format_lines("Miller Staking"),
        format_lines("Kroger Gardis and Regas LLC Marketing Manager"),
        format_lines("Kroger Gardis and Regas LLC Education Team Summer Law Clerk"),
        format_lines("Wabash College Advancement Intern"),
        format_lines("Various Indiana Public Schools Substitute Teacher"),
        format_lines("Phi Delta Theta, IN Beta"),
        format_lines("Miami County Agricultural Association Webmaster"),
        relevant_skills.join(", ")
    )
}