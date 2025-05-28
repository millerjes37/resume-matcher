#import "@preview/basic-resume:0.2.8": *

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
- Researched emerging technologies in #strong[Rust] and web development for continuous improvement.
- Developed a full-stack SAAS application using #strong[Rust] and Dioxus for lobbyists.
- Utilized asynchronous #strong[Rust] programming to handle high-concurrency operations.
- Provided technical support and training to clients and internal teams.

#project(
  role: "Managing Member",
  name: "Miller Staking",
  dates: dates-helper(start-date: "April 2023", end-date: "Present"),
)
- Assisted in the development of marketing materials and online presence.
- Maintained inventory of supplies and equipment for daily operations.
- Utilized project management to ensure timely service delivery.
- Onboarded new clients and provided orientation to services and procedures.

== Work Experience
#work(
  title: "Marketing Manager",
  location: "Indianapolis, IN",
  company: "Kroger Gardis and Regas LLC",
  dates: dates-helper(start-date: "May 2023", end-date: "September 2023"),
)
- Oversaw the design and production of print collateral, including brochures and advertisements.
- Developed and managed the marketing budget, ensuring efficient allocation of resources.

#work(
  title: "Education Team Summer Law Clerk",
  location: "Indianapolis, IN",
  company: "Kroger Gardis and Regas LLC",
  dates: dates-helper(start-date: "May 2023", end-date: "September 2023"),
)
- Assisted with the drafting of school board policies and administrative guidelines.
- Assisted in the preparation for client meetings, hearings, and depositions.
- Attended client consultations and internal strategy sessions with the education law team.
- Participated in team meetings to discuss case strategies and ongoing projects.

#work(
  title: "Advancement Intern",
  location: "Crawfordsville, IN",
  company: "Wabash College",
  dates: dates-helper(start-date: "September 2022", end-date: "May 2023"),
)


#work(
  title: "Substitute Teacher",
  location: "Indiana",
  company: "Various Indiana Public Schools",
  dates: dates-helper(start-date: "September 2022", end-date: "May 2023"),
)


== Professional Development
#work(
  title: "Advisor, Vice President, Recruitment Chair, Risk Manager (non-concurrent)",
  location: "Crawfordsville, IN",
  company: "Phi Delta Theta, IN Beta",
  dates: dates-helper(start-date: "November 2019", end-date: "November 2022"),
)


#work(
  title: "Webmaster",
  location: "Converse, IN",
  company: "Miami County Agricultural Association",
  dates: dates-helper(start-date: "November 2017", end-date: "Present"),
)


== Skills
#set align(center)
Rust, Python, NLP
