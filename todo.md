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

- Ignore models with disabled shadown casting when determining shadow map bounds.

## Physics

- Support disabling rigid bodies.

- Add more constraints.

- Implement Voronoi fracturing.

- Implement N-body gravity simulation using multipole expansion for the gravitational field of extended objects and a Barnes-Hut tree as acceleration structure.

- Correct gravitational field inside voxel objects.

- Improve physics stability (avoid crash when small pieces explode with NaN).

- Continuous collision detection.

- Capsule collider shape.

- Prevent colliders from being pushed towards voxel object interior when colliding with voxels.

## Voxels

- Add smoothing pass after Surface Nets.

- Per surface voxel state (e.g. temperature).

- Make voxels carry ID for region association? Would enable detection of disconnected regions without empty voxels between, useful for e.g. Voronoi fracturing. Optionally, implement dedicated disconnected region detection with explicit region labels that could be sourced directly from Voronoi computation.

- Define empty voxels as having signed distance larger than voxel radius, rather than just having positive signed distance.

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

- Make relevant `impact_ecs` types accept allocator.

## ECS

- Support querying only entities where certain components have changed.

- Consider reducing usage of `RwLock` in `impact_ecs`. Investigate scheduler as an alternative to locks.

- Check to see if entities could be created before actual creation so that we don't run setup and then fail entity creation.

## Scene

- Ray intersection queries.

## Misc

- Fix black square flashes due to bloom filter propagating NaNs.

- Fix intermittent black triangles for voxel objects.

- Fix tiny gaps between chunk meshes due to numerical imprecision.

- Fix surface-grazing chunked marked as empty after voxel absorption even though the surface protrudes slightly into the chunk and no voxels were absorbed in or near the chunk (run at commit #9f72fb5f to reproduce).


# Shadow map bounding

Query the BVH with camera view frustum and compute camera space AABB encompassing all visible models.

For each omnidirectional light:
Let the central axis of the negative z cubemap face point towards the center of the AABB for visible models. This defines light space.
Compute/estimate the horizontal and vertical angular bounds as well as maximum depth of the visible models OBB in light space.
Use this to define a frustum with zero near distance encompassing all visible models seen from the light.
Query the BVH with this frustum and compute the shortest and longest distance to non-exterior models that are also shadowing.
Keep a record of which shadowing models were found.
This gives the shadow cubemap near and far distance.
Use the angular bounds to determine which cubemap faces could see visible models, and query the BVH with each of these.
For each face query, buffer model to light transforms for shadowing models that were recorded as found in the initial frustum query (others will not shadow any visible models).

Partition depths for shadow map cascades can be computed once (per frame) from the min and max z-coordinate of the camera-space AABB for visible models.

For each unidirectional light:
Compute the light space cascade AABB encompassing each part (sub-frustum) of the partitioned view frustum, but extending to the outer bound of the scene in the direction against the light.
For each cascade:
Query the BVH with the (in world space) cascade OBB. Gather the IDs of encountered models that are also shadowing, and track the extremal coordinates of their light space bounds.
Use the extremal light space bounds to define the cascade's orthographic transform.
Go through the gathered model IDs and buffer their model to light transforms.
