use crate::cleanup::types::{BuildCacheStats, BuildCacheItem};
use anyhow::Result;
use bollard::Docker;

/// Analyze Docker build cache
pub async fn analyze_build_cache(docker: &Docker) -> Result<BuildCacheStats> {
    // Get build cache disk usage
    let df = docker.df().await?;

    let mut stats = BuildCacheStats::default();

    if let Some(build_cache) = df.build_cache {
        for cache_item in build_cache {
            let size = cache_item.size.unwrap_or(0).max(0) as u64;
            let in_use = cache_item.in_use.unwrap_or(false);
            let shared = cache_item.shared.unwrap_or(false);

            stats.total_size_bytes += size;
            if !in_use {
                stats.reclaimable_bytes += size;
            }

            stats.items.push(BuildCacheItem {
                id: cache_item.id.unwrap_or_default(),
                cache_type: cache_item.typ.map(|t| format!("{:?}", t)).unwrap_or_else(|| "unknown".to_string()),
                size_bytes: size,
                created_timestamp: cache_item.created_at.and_then(|dt| {
                    chrono::DateTime::parse_from_rfc3339(&dt)
                        .ok()
                        .map(|d| d.timestamp())
                }).unwrap_or(0),
                last_used_timestamp: cache_item.last_used_at.and_then(|dt| {
                    chrono::DateTime::parse_from_rfc3339(&dt)
                        .ok()
                        .map(|d| d.timestamp())
                }),
                in_use,
                shared,
            });
        }
    }

    // Sort by size descending
    stats.items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    Ok(stats)
}

/// Prune build cache (removes unused cache only)
/// Note: Build cache pruning is not directly supported by Bollard's Docker API
/// This would need to be done via CLI: `docker builder prune`
pub async fn prune_build_cache(_docker: &Docker) -> Result<PruneStats> {
    // TODO: Implement via system exec or wait for Bollard API support
    // For now, return zero space reclaimed
    Ok(PruneStats {
        space_reclaimed: 0,
    })
}

#[derive(Debug)]
pub struct PruneStats {
    pub space_reclaimed: u64,
}
