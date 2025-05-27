pub fn compute_cosine_similarity(vec1: &[f32], vec2: &[f32]) -> f32 {
    let dot: f32 = vec1.iter().zip(vec2).map(|(a, b)| a * b).sum();
    let norm1: f32 = (vec1.iter().map(|x| x * x).sum::<f32>()).sqrt();
    let norm2: f32 = (vec2.iter().map(|x| x * x).sum::<f32>()).sqrt();
    dot / (norm1 * norm2).max(1e-10)
}
