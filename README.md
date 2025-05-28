# ATS-like scoring of work expirience Lines

I beleive it is disingenuous to us AI to write out your resume. As a business owner I am innundated with resumes and coverletters from young people who clearly used AI to currate their resume. What sucks is, there is, without using an LLM, a monumental disparity in the work required to apply to a given job and the work required to parse a job. Companies who empploy an ATS drive this issue by screening resumes for a match to the provided job description. This project attempts to strike a middle ground.

By Using the job description and scoring indivudual prewritten (by hand) lines that describe the given work expirience. This will automatically surface and populate the bullets that most closely match a string in the job description. We incorperate a number of different common scoring algorithims in this project to create a wholistic look in order to provide the best possible score of lines accross scoring systems.

In the end, everything in the resume has been hand-written by me, Jackson Miller. But the hand written paragraphs are selected via scoring of each individual line. This improves the overall efficency of writing resumes and appling to jobs and allows me to balance quality of what I am submitting and speed at which I can apply to jobs.


This project automates the creation of tailored resumes and cover letters, helps track job applications, and includes a web scraper to fetch job descriptions.

## Features

*   **Tailored Document Generation:** Generates `.typ` (Typst) files for resumes and cover letters based on job descriptions.
*   **Skill Highlighting:** Automatically highlights relevant skills in the generated resume.
*   **PDF Output:** Compiles Typst files into `.pdf` format.
*   **Web Scraping:** Fetches job descriptions from a URL.
*   **Organized Output:** Saves generated files in a structured directory: `output/{Company}-{Role}-{Date}/{src|export|prep_materials}`.
*   **Tracking Numbers:** Assigns unique tracking numbers to each application.
*   **Application Logging:** Records all generated applications in an `applications.csv` file.
*   **CLI Dashboard:** Provides a terminal-based dashboard (built with Ratatui) to view and track applications.
*   **Supporting Documents:** Creates a `prep_materials` directory for each application to store notes, emails, etc.

## Directory Structure

-   `job_descriptions/`: Place your job description files here (e.g., `Company-Role.txt` or `Company-Role.md`). Scraped job descriptions are also saved here.
-   `output/`: Contains all generated application packages.
    -   `{Company}-{Role}-{Date}/`: Main folder for a specific application.
        -   `src/`: Contains the generated Typst source files (`.typ`).
            -   `Resume_{Company}_{Role}_{Tracking#}.typ`
            -   `CoverLetter_{Company}_{Role}_{Tracking#}.typ`
        -   `export/`: Contains the compiled PDF files (`.pdf`).
            -   `Resume_{Company}_{Role}_{Tracking#}.pdf`
            -   `CoverLetter_{Company}_{Role}_{Tracking#}.pdf`
        -   `prep_materials/`: A place for you to store notes, emails, or other preparation documents related to this application.
-   `src/`: Contains the Rust source code.
    -   `main.rs`: Main application logic.
    *   `scraper.rs`: Web scraping functionality.
    *   `dashboard.rs`: Ratatui dashboard UI and logic.
    *   `resume-lines.json`: Your core resume content (experiences, bullet points).
-   `applications.csv`: Logs details of all generated applications.
-   `Cargo.toml`: Rust project configuration.
-   `README.md`: This file.

## Prerequisites

1.  **Rust:** Ensure Rust is installed. You can get it from [rust-lang.org](https://www.rust-lang.org/).
2.  **Typst:** The Typst CLI is required for compiling documents to PDF. Installation instructions can be found at [typst.app](https://typst.app/docs/guides/install/). Make sure `typst` is in your system's PATH.

## Installation

1.  Clone the repository:
    ```bash
    git clone <repository_url>
    cd <repository_directory>
    ```
2.  Build the project:
    ```bash
    cargo build
    ```
    For a release version (recommended for speed):
    ```bash
    cargo build --release
    ```
    The executable will be in `target/debug/resume-matcher` or `target/release/resume-matcher`.

## Usage

### 1. Processing Local Job Descriptions

*   Place job description files (e.g., `ExampleCorp-SoftwareEngineer.txt` or `.md`) in the `job_descriptions/` directory.
*   Run the application:
    ```bash
    cargo run 
    # Or, if built for release:
    # ./target/release/resume-matcher
    ```
*   The program will process each file, generate documents, and save them in the `output/` directory.

### 2. Scraping a Job Description from a URL

*   Run the application (as above).
*   It will first prompt you to enter a job description URL:
    ```
    Enter a job description URL to scrape (or press Enter to skip):
    ```
*   If you provide a URL:
    *   The system will attempt to scrape it.
    *   You will then be prompted to enter the Company and Role for the job to correctly name the scraped file (which will be saved in `job_descriptions/`).
    ```
    Enter company name for the scraped job: ExampleCorp
    Enter role for the scraped job: DataAnalyst
    ```
    *   The scraped job description will then be processed like a local file.

### 3. Viewing the Application Dashboard

*   To view the list of generated applications, run:
    ```bash
    cargo run -- dashboard
    # Or, if built for release:
    # ./target/release/resume-matcher dashboard
    ```
*   **Dashboard Controls:**
    *   `Up Arrow`/`Down Arrow`: Navigate through the list of applications.
    *   `q`: Quit the dashboard.

## Customization

*   **Resume Content:** Edit `resume-lines.json` to update your resume's bullet points, experiences, and skills. This is the primary source for tailoring your resume.
*   **Typst Templates:** The Typst templates for the resume and cover letter are currently embedded within `src/main.rs` in the `generate_typst_resume` and `generate_typst_cover_letter` functions. You can modify these Rust string literals to change the layout or static content.

## Troubleshooting

*   **Typst Compilation Errors:** If you see errors like "Typst command failed" or "Failed to execute Typst command," ensure:
    1.  Typst is correctly installed.
    2.  The `typst` command is accessible from your system's PATH.
    3.  The Typst files (`.typ`) generated in the `src` directory do not have syntax errors (check console output from the main program for any Typst error messages if PDF generation fails).
*   **Web Scraper Issues:** The web scraper uses general selectors and may not work perfectly for all job websites due to varying HTML structures. If scraping fails or extracts poor quality text, you may need to manually copy the job description into a `.txt` or `.md` file in the `job_descriptions/` directory.
