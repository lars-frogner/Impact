# Impact
A rendering and physics engine written in C++. 

### Features
- 2D and 3D geometry library, including a triangle mesh class for representing arbitrary model shapes.
- Point lights, directional lights and area lights.
- Materials with a Phong BSDF, with support for mirror reflection and transparency.
- Textures, normal mapping and displacement mapping.
- Multiple rendering methods for direct lighting: Gouraud shading, ray tracing and rasterization.
- Global illumination and geometrical optics through path tracing.
- Real-time animation with interactive camera control, simulation control and video recording.
- Physical simulation of (macroscopic) particles, with a range of possible forces and constraints.

### External libraries
Impact uses [Armadillo](http://arma.sourceforge.net/) with [LAPACK](http://www.netlib.org/lapack/) and [BLAS](http://www.netlib.org/blas/) for some of the linear algebra, in addition to [OpenGL](https://www.opengl.org/) with [GLUT](https://www.opengl.org/resources/libraries/glut/) for displaying the rendered images.

### Screenshots
**Cornell box (path traced)**
![path traced spheres](/Screenshots/cornell_box_with_normal_map.png?raw=true "Cornell box")


**Transparent, diffuse and mirror-reflective sphere (path traced)**
![path traced spheres](/Screenshots/path_tracing_test.png?raw=true "Path traced spheres")


**Utah teapot (ray traced)**
![red teapot](/Screenshots/2017_09_04_red_teapot.png?raw=true "Red teapot")


**Snapshots from particle simulation (ray traced)**
![physics simulation](/Screenshots/balls_full.png?raw=true "Snapshots from physics simulation")