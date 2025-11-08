use crate::cleanup::types::{LayerAnalysis, SharedLayer};
use anyhow::Result;
use bollard::Docker;
use bollard::image::ListImagesOptions;
use std::collections::HashMap;

/// Analyze image layers to identify sharing and efficiency
pub async fn analyze_layers(docker: &Docker) -> Result<LayerAnalysis> {
    let images = docker.list_images(None::<ListImagesOptions<String>>).await?;

    // Map: layer_id -> (size, Vec<image_names>)
    let mut layer_usage: HashMap<String, (i64, Vec<String>)> = HashMap::new();
    let mut total_image_sizes = 0i64;

    for image in &images {
        let image_name = image
            .repo_tags
            .first()
            .cloned()
            .unwrap_or_else(|| format!("<none>:{}", &image.id[7..19]));

        // Skip dangling images for layer analysis
        if image_name.contains("<none>") {
            continue;
        }

        total_image_sizes += image.size;

        // Inspect image to get layer information
        if let Ok(inspect) = docker.inspect_image(&image.id).await {
            if let Some(root_fs) = inspect.root_fs {
                if let Some(ref layers) = root_fs.layers {
                    let layer_count = layers.len();
                    for layer_id in layers {
                        let entry = layer_usage.entry(layer_id.clone()).or_insert((0, Vec::new()));
                        entry.1.push(image_name.clone());

                        // Size estimation - divide image size by number of layers
                        // This is approximate since we can't get individual layer sizes easily
                        if layer_count > 0 {
                            entry.0 = image.size / layer_count as i64;
                        }
                    }
                }
            }
        }
    }

    // Calculate shared layers (used by 2+ images)
    let mut shared_layers = Vec::new();
    let mut total_shared_bytes = 0u64;
    let mut total_unique_bytes = 0u64;

    for (layer_id, (size, images_using)) in layer_usage {
        let size_bytes = size.max(0) as u64;

        if images_using.len() > 1 {
            // Shared layer
            total_shared_bytes += size_bytes;
            shared_layers.push(SharedLayer {
                layer_id: layer_id[7..19].to_string(), // Short ID
                size_bytes,
                shared_by_count: images_using.len(),
                images_using: images_using.clone(),
                created_timestamp: 0, // Not easily available
            });
        } else {
            // Unique layer
            total_unique_bytes += size_bytes;
        }
    }

    // Sort shared layers by size descending
    shared_layers.sort_by(|a, b| {
        b.size_bytes
            .cmp(&a.size_bytes)
            .then(b.shared_by_count.cmp(&a.shared_by_count))
    });

    // Calculate efficiency: What % of space is saved by layer sharing?
    // If all layers were unique: total_image_sizes
    // With sharing: total_shared_bytes + total_unique_bytes
    let total_actual = total_shared_bytes + total_unique_bytes;
    let efficiency_percent = if total_actual > 0 {
        (1.0 - (total_actual as f64 / total_image_sizes.max(1) as f64)) * 100.0
    } else {
        0.0
    };

    Ok(LayerAnalysis {
        shared_layers,
        total_shared_bytes,
        total_unique_bytes,
        efficiency_percent,
    })
}
