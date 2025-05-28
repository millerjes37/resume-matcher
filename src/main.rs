use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType, SentenceEmbeddingsModel};
use rust_bert::pipelines::ner::NERModel;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::collections::HashMap;
use chrono::Local;
use regex::Regex;
use std::io::{self}; // Add io to use statements
use std::process::Command; // Add Command to use statements
use csv::WriterBuilder; // Add for CSV logging
use std::path::PathBuf;  // Add for CSV logging

mod utils;
mod graph_scoring;
mod scoring;
mod tfidf;
mod scraper; // Add mod scraper
mod dashboard; // Add mod dashboard

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
    // --- Dashboard Launch ---
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "dashboard" {
        println!("Launching dashboard...");
        if let Err(e) = dashboard::run_dashboard() {
            eprintln!("Dashboard error: {}", e);
        }
        return Ok(()); // Exit after dashboard runs
    }
    // --- End Dashboard Launch ---

    println!("Initializing models...");
    
    // Initialize models
    let bert_model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .create_model()?;
    let ner_model = NERModel::new(Default::default())?;

    // --- Scraper Integration ---
    println!("Enter a job description URL to scrape (or press Enter to skip):");
    let mut url_input = String::new();
    io::stdin().read_line(&mut url_input)?;
    let url_input = url_input.trim();

    if !url_input.is_empty() {
        println!("Scraping URL: {}", url_input);
        match scraper::scrape_job_url(url_input) {
            Ok(scraped_content) => {
                println!("Successfully scraped content.");
                // Prompt for company and role
                let mut company_input = String::new();
                let mut role_input = String::new();
                
                print!("Enter company name for the scraped job: ");
                io::stdout().flush()?; // Ensure prompt is shown before input
                io::stdin().read_line(&mut company_input)?;
                let company = company_input.trim();

                print!("Enter role for the scraped job: ");
                io::stdout().flush()?;
                io::stdin().read_line(&mut role_input)?;
                let role = role_input.trim();

                if company.is_empty() || role.is_empty() {
                    eprintln!("Company and Role are required to save scraped content. Skipping saving.");
                } else {
                    // Ensure job_descriptions directory exists
                    fs::create_dir_all("job_descriptions")?; 
                    let timestamp = Local::now().format("%Y%m%d%H%M%S").to_string();
                    let filename = format!("job_descriptions/{}-{}-{}.md", company, role, timestamp);
                    match fs::write(&filename, &scraped_content) {
                        Ok(_) => println!("Scraped content saved to {}", filename),
                        Err(e) => eprintln!("Error saving scraped content: {}", e),
                    }
                }
            }
            Err(e) => {
                eprintln!("Error scraping URL: {}", e);
            }
        }
    }
    // --- End Scraper Integration ---

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

    // Generate tracking number and date
    let tracking_number = Local::now().format("%Y%m%d%H%M%S").to_string();
    let date_str = Local::now().format("%Y-%m-%d").to_string(); // Already used for folder name

    // Define output paths
    let base_output_dir = format!("output/{}-{}-{}", company, role, date_str);
    let src_dir = format!("{}/src", base_output_dir);
    let export_dir = format!("{}/export", base_output_dir);

    // Create directories
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&export_dir)?;
    let prep_materials_dir = format!("{}/prep_materials", base_output_dir);
    fs::create_dir_all(&prep_materials_dir)?;
    println!("Created prep materials directory: {}", prep_materials_dir);

    // Generate Typst resume
    let typst_content = generate_typst_resume(&selected_lines, &relevant_skills); // This `relevant_skills` is now correctly defined above
    
    // Corrected output path for the resume
    let resume_output_path = format!("{}/Resume_{}_{}_{}.typ", src_dir, company, role, tracking_number);
    let mut file = File::create(&resume_output_path)?;
    file.write_all(typst_content.as_bytes())?;

    println!("Generated resume: {}", resume_output_path);

    // Call generate_typst_cover_letter
    // User details are hardcoded here as they are in generate_typst_resume (after its refactor)
    // In a larger application, these would ideally come from a config or a shared struct.
    let user_name_cv = "Jackson Miller";
    let user_email_cv = "jackson@civitas.ltd";
    let user_phone_cv = "+1 (260) 377-9575";
    let user_linkedin_cv = "linkedin.com/in/jackson-e-miller";
    let user_github_cv = "github.com/millerjes37";
    let user_site_cv = "jackson-miller.us";

    let cover_letter_content = generate_typst_cover_letter(
        &company,
        &role,
        &date_str, // already defined from previous steps
        &tracking_number, // already defined from previous steps
        user_name_cv,
        user_email_cv,
        user_phone_cv,
        user_linkedin_cv,
        user_github_cv,
        user_site_cv
    );

    // Save the Cover Letter
    let cover_letter_output_path = format!("{}/CoverLetter_{}_{}_{}.typ", src_dir, company, role, tracking_number);
    let mut cl_file = File::create(&cover_letter_output_path)?;
    cl_file.write_all(cover_letter_content.as_bytes())?;
    println!("Generated cover letter: {}", cover_letter_output_path);

    // Compile Resume to PDF
    let resume_pdf_filename = format!("Resume_{}_{}_{}.pdf", company, role, tracking_number);
    let resume_pdf_output_path = format!("{}/{}", export_dir, resume_pdf_filename);

    println!("Compiling resume to PDF: {}", resume_pdf_output_path);
    let compile_resume_status = Command::new("typst")
        .arg("compile")
        .arg(&resume_output_path) // Input .typ file
        .arg(&resume_pdf_output_path) // Output .pdf file
        .status();

    match compile_resume_status {
        Ok(status) => {
            if status.success() {
                println!("Successfully compiled resume to PDF.");
            } else {
                eprintln!("Error compiling resume: Typst command failed with status: {}. Ensure Typst is installed and in PATH.", status);
            }
        }
        Err(e) => {
            eprintln!("Failed to execute Typst command for resume: {}. Ensure Typst is installed and in PATH.", e);
        }
    }

    // Compile Cover Letter to PDF
    let cover_letter_pdf_filename = format!("CoverLetter_{}_{}_{}.pdf", company, role, tracking_number);
    let cover_letter_pdf_output_path = format!("{}/{}", export_dir, cover_letter_pdf_filename);
    
    println!("Compiling cover letter to PDF: {}", cover_letter_pdf_output_path);
    let compile_cl_status = Command::new("typst")
        .arg("compile")
        .arg(&cover_letter_output_path) // Input .typ file
        .arg(&cover_letter_pdf_output_path) // Output .pdf file
        .status();

    match compile_cl_status {
        Ok(status) => {
            if status.success() {
                println!("Successfully compiled cover letter to PDF.");
            } else {
                eprintln!("Error compiling cover letter: Typst command failed with status: {}. Ensure Typst is installed and in PATH.", status);
            }
        }
        Err(e) => {
            eprintln!("Failed to execute Typst command for cover letter: {}. Ensure Typst is installed and in PATH.", e);
        }
    }

    // CSV Logging
    let csv_path = PathBuf::from("applications.csv");
    let file_exists = csv_path.exists();

    let csv_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(&csv_path)?;

    let mut wtr = WriterBuilder::new().has_headers(!file_exists).from_writer(csv_file);

    // Headers are: TrackingNumber, Company, Role, ApplicationDate, ResumeTypPath, CoverLetterTypPath, ResumePdfPath, CoverLetterPdfPath, Status
    wtr.serialize((
        &tracking_number,
        &company,
        &role,
        &date_str, // Processing date
        &resume_output_path,
        &cover_letter_output_path,
        &resume_pdf_output_path, 
        &cover_letter_pdf_output_path,
        "Generated", // Initial status
    ))?;
    wtr.flush()?;
    println!("Logged application details to {}", csv_path.display());

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

    let user_name = "Jackson Miller";
    let user_location = "Wabash, IN";
    let user_email = "jackson@civitas.ltd";
    let user_github = "github.com/millerjes37";
    let user_linkedin = "linkedin.com/in/jackson-e-miller";
    let user_phone = "+1 (260) 377-9575";
    let user_personal_site = "jackson-miller.us";

    format!(
        r##"#import "@preview/basic-resume:0.2.8": *

#let name = "{}"
#let location = "{}"
#let email = "{}"
#let github = "{}"
#let linkedin = "{}"
#let phone = "{}"
#let personal-site = "{}"

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
        user_name,
        user_location,
        user_email,
        user_github,
        user_linkedin,
        user_phone,
        user_personal_site,
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

fn generate_typst_cover_letter(
    company: &str,
    role: &str,
    current_date_str: &str,
    tracking_number: &str,
    user_name: &str, 
    user_email: &str,
    user_phone: &str,
    user_linkedin: &str,
    user_github: &str,
    user_site: &str
) -> String {
    // A simple, generic cover letter template
    format!(
        r##"#import "@preview/basic-cv:0.2.8": * // Assuming similar library or create a new one
        // Or use basic Typst document setup if a CV-specific one isn't ideal for letters.
        // For simplicity, let's use a structure similar to the resume for contact info.
        
        #let name = "{}"            // Maps to user_name
        #let email = "{}"           // Maps to user_email
        #let phone = "{}"           // Maps to user_phone
        #let linkedin = "{}"        // Maps to user_linkedin
        #let github = "{}"          // Maps to user_github
        #let personal_site = "{}"   // Maps to user_site
        #let today_date = "{}"      // Maps to current_date_str
        #let company_name = "{}"    // Maps to company
        #let job_role = "{}"        // Maps to role
        #let trk_num = "{}"         // Maps to tracking_number

        #show: letter.with( // 'letter' might need to be a custom show rule or use default page setup
            author: name,
            author-contact-details: {{
                table(
                    columns: 2,
                    align: (left, left),
                    row-gutter: 0.65em,
                    [], // Icon (optional)
                    [#link("mailto:" + email)[{email}]], // Corrected: # removed from {email}
                    [], 
                    [#link("tel:" + phone)[{phone}]], // Corrected: # removed from {phone}
                    [], 
                    [#link("https://" + linkedin)[{linkedin}]], // Corrected: # removed from {linkedin}
                    [], 
                    [#link("https://" + github)[{github}]], // Corrected: # removed from {github}
                    [], 
                    [#link("https://" + personal_site)[{personal_site}]], // Corrected: # removed from {personal_site}
                )
            }},
            recipient-details: {{
                [The Hiring Team]                     [#strong[#company_name]]
                // [123 Main Street]                     // [Anytown, ST 12345]
            }},
            date: today_date,
            subject: [#strong[Application for #job_role Position (Ref: #trk_num)]],
            body: {{
                [Dear Hiring Team at #strong[#company_name],] // Use #company_name

                [I am writing to express my keen interest in the #strong[#job_role] position at #strong[#company_name], as advertised on [Platform - e.g., LinkedIn, company website - placeholder]. Having followed #strong[#company_name]'s work in [Industry/Area - placeholder] for some time, I am impressed by [Specific aspect of company - placeholder].] // Use #job_role, #company_name (3 times)

                [My background in [Your Key Skill/Area 1] and [Your Key Skill/Area 2], combined with my experience in [Relevant Experience Area], aligns well with the requirements outlined in the job description. I am particularly adept at [Mention a key responsibility from JD or a relevant achievement].]

                // This section can be manually customized later or programmatically enhanced
                // For example, one could insert some of the top selected resume lines here if relevant.
                // For now, it's a generic paragraph.

                [I am confident that my skills and enthusiasm would make me a valuable asset to your team. I have attached my resume for your review, which further details my qualifications and accomplishments (Tracking: #trk_num).]

                [Thank you for your time and consideration. I look forward to the possibility of discussing this exciting opportunity with you.]

                [Sincerely,]                     [#name] // Use #name
            }}
        )
        
        // Minimal 'letter' show rule definition (if not using a template that provides one)
        #let letter(
            author:,
            author-contact-details:,
            recipient-details:,
            date:,
            subject:,
            body:
        ) = {{
            set text(font: "Linux Libertine", size: 11pt)
            set page(margin: (top: 1.5in, bottom: 1in, left: 1in, right: 1in))

            grid(
                columns: (1fr, 2fr),
                rows: auto,
                gutter: 1em,
                align(right, author-contact-details), 
                [] 
            )
            move(dy: -1.5em, dx: 0em, align(left, author)) 

            move(dy: 1em, recipient-details)
            move(dy: 1em, date)
            move(dy: 1em, subject)
            move(dy: 1em, body)
        }}
        "##,
        user_name,        // For #let name
        user_email,       // For #let email
        user_phone,       // For #let phone
        user_linkedin,    // For #let linkedin
        user_github,      // For #let github
        user_site,        // For #let personal_site
        current_date_str, // For #let today_date
        company,          // For #let company_name
        role,             // For #let job_role
        tracking_number   // For #let trk_num
    )
}