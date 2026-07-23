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

- Investigate if there is a synchronization issue (wgpu bug?) that allows the lighting pass to read from the linear depth buffer before the geometry pass has written to it, leading to zero depth and subsequent NaNs.

## Physics

- Support disabling rigid bodies.

- Add more constraints.

- Implement Voronoi fracturing.

- Implement N-body gravity simulation using multipole expansion for the gravitational field of extended objects and a Barnes-Hut tree as acceleration structure.

- Correct gravitational field inside voxel objects.

- Improve physics stability (avoid crash when small pieces explode with NaN).

## Voxels

- Per surface voxel state (e.g. temperature).

## Generation

- Implement graph based system for generating voxel types (probably integrated with SDF graph).

## Performance

- Pool per-object voxel GPU buffers into shared arenas to enable one culling dispatch and one multi-draw per view (see `docs/voxel_gpu_buffer_pooling.md`).

- Use single bind group for G-buffer attachments.

- Investigate ways to improve shadow map update performance (check out multiview render passes).

- Consider using `wgpu::TextureFormat::Rg11b10Float` for appropriate attachments.

- Handle rendering of single-chunk voxel objects separately in a more lightweight manner.

- Investigate where arena allocation could be beneficial.

- Add custom allocator support for `AlignedByteVec`.

- Make relevant `impact_ecs` types accept allocator.

- Make `VoxelObjectManager` maintain pool of reusable `VoxelObjectBuffers`.

## ECS

- Support querying only entities where certain components have changed.

- Consider reducing usage of `RwLock` in `impact_ecs`. Investigate scheduler as an alternative to locks.

- Check to see if entities could be created before actual creation so that we don't run setup and then fail entity creation.

## Scene

- Ray intersection queries.

## Misc

- Fix acceleration stuck on non-zero in game.
