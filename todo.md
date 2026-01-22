# TODO

## Resources

- Add some form of dependency tracking for resource registries to automatically evict unused resources.

- Instead of loading resources directly from files, create a database for processed resources. Importing assets into the resource database is done offline. At runtime, the engine only loads from the resource database.

## Graphics

- Support clicking on rendered entities to display info based on their components.

- Look into clustered shading.

- Create general-purpose debug rendering API invokable from anywhere.

- Centralize assignment of binding locations?

- Check out Reddit shadow mapping article.

- Look into mesh shading (could be useful for voxel rendering).

- Consider Texel Snapping for shadow map cascades.

## Physics

- Support disabling rigid bodies.

- Add more constraints.

- Implement Voronoi fracturing.

- Implement N-body gravity simulation using multipole expansion for the gravitational field of extended objects and a Barnes-Hut tree as acceleration structure.

- Improve physics stability (avoid crash when small pieces explode with NaN).

- Adjust voxel collider sphere radius based on signed distance.

- Voxel absorption - limit signed distance based on distance from absorber.

## Generation

- Implement graph based system for generating voxel types (probably integrated with SDF graph).

## Performance

- Use single bind group for G-buffer attachments.

- Investigate ways to improve shadow map update performance (check out multiview render passes).

- Consider using `wgpu::TextureFormat::Rg11b10Float` for appropriate attachments.

- Handle rendering of single-chunk voxel objects separately in a more lightweight manner.

- Consider using Welzl's algorithm to compute better bounding spheres.

- Investigate where arena allocation could be beneficial.

- Add acceleration structure for broad phase collision detection.

- Add custom allocator support for `AlignedByteVec`.

## ECS

- Support querying only entities where certain components have changed.

- Consider reducing usage of `RwLock` in `impact_ecs`. Investigate scheduler as an alternative to locks.

## Roc

- Support state/context in Roc script.

## Misc

- Fix black square flashes due to bloom filter propagating NaNs.

- Fix intermittent black triangles for voxel objects.

- Fix tiny gaps between chunk meshes due to numerical imprecision.

- Fix sporadic `assertion failed: radius >= 0.0` from `sphere.rs` (probably when absorbing voxels with sphere).

- Investigate "floating" non-empty chunks (separated from main body with empty chunks but not disconnected). Is separation by whole empty chunks handled?
