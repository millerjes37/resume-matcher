use reqwest;
use scraper::{Html, Selector};

pub fn scrape_job_url(url: &str) -> anyhow::Result<String> {
    // Fetch the HTML content from the URL
    let response_text = reqwest::blocking::get(url)?.text()?;

    // Parse the HTML
    let document = Html::parse_document(&response_text);

    // Define selectors for common job description containers.
    // This is a basic set and might need refinement.
    // We'll try to find common parent elements that might contain the main content.
    let selectors_str = vec![
        "article", "main", ".job-description", ".job-details", // common semantic tags or class names
        "#job-description", "#job-details", // common IDs
        "div[class*='description']", "div[class*='details']" 
    ];

    let mut extracted_text = String::new();

    for sel_str in selectors_str {
        if let Ok(selector) = Selector::parse(sel_str) {
            for element in document.select(&selector) {
                // Extract all text nodes within the element, join them.
                let element_text = element.text().collect::<Vec<_>>().join(" ");
                extracted_text.push_str(&element_text);
                extracted_text.push_str("\n\n"); // Add some separation
            }
            // If we found content with a selector, we might assume it's the main one.
            // This is a very simple heuristic.
            if !extracted_text.is_empty() {
                break; 
            }
        }
    }
    
    // Fallback: if no specific container found, try to get all <p> text
    if extracted_text.is_empty() {
        if let Ok(p_selector) = Selector::parse("p") {
            for element in document.select(&p_selector) {
                extracted_text.push_str(&element.text().collect::<Vec<_>>().join(" "));
                extracted_text.push_str("\n");
            }
        }
    }

    if extracted_text.is_empty() {
         // As an ultimate fallback, strip all tags (basic approach)
         let body_selector = Selector::parse("body").unwrap(); // body should always exist
         if let Some(body_element) = document.select(&body_selector).next() {
             extracted_text = body_element.text().collect::<Vec<_>>().join(" ");
         } else {
             return Err(anyhow::anyhow!("Could not extract any meaningful content from the page."));
         }
    }
    
    // Basic cleaning: trim whitespace and replace multiple newlines/spaces
    let cleaned_text = extracted_text.trim().to_string();
    // This regex part might be tricky with escaping in the subtask string,
    // let's suggest a simpler replace for now or the worker can refine it.
    // For now, let's rely on simple trimming and the structure from selectors.
    // Ok(cleaned_text.split_whitespace().collect::<Vec<_>>().join(" ")) // Consolidate whitespace

    Ok(cleaned_text)
}
