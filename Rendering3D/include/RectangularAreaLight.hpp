#pragma once
#include "precision.hpp"
#include "AreaLight.hpp"
#include "Color.hpp"
#include "SurfaceElement.hpp"
#include "Point3.hpp"
#include "Vector3.hpp"
#include "CoordinateFrame.hpp"
#include "TriangleMesh.hpp"
#include "LinearTransformation.hpp"
#include "AffineTransformation.hpp"

namespace Impact {
namespace Rendering3D {

class RectangularAreaLight : public AreaLight {
    
private:
    typedef Geometry3D::Point Point;
    typedef Geometry3D::Vector Vector;
    typedef Geometry3D::CoordinateFrame CoordinateFrame;
    typedef Geometry3D::TriangleMesh TriangleMesh;
    typedef Geometry3D::LinearTransformation LinearTransformation;
    typedef Geometry3D::AffineTransformation AffineTransformation;
    
    Point _origin;
    Vector _width_vector, _height_vector, _direction;
    Power _power;
    imp_float _width, _height;
    imp_uint _n_samples;

public:
    RectangularAreaLight(const Point& new_center,
                         const Vector& direction,
                         const Vector& new_width_vector,
                         imp_float height,
                         const Power& new_power,
                         imp_uint new_n_samples);
    
    RectangularAreaLight(const Point& new_center,
                         const Point& point_in_direction,
                         const Vector& new_width_vector,
                         imp_float height,
                         const Power& new_power,
                         imp_uint new_n_samples);

    CoordinateFrame getCoordinateFrame() const;
    imp_float getSurfaceArea() const;
    imp_uint getNumberOfSamples() const;
	TriangleMesh getMesh() const;

    Point getRandomPoint() const;

    SurfaceElement getRandomSurfaceElement() const;

    Power getTotalPower() const;

	void setCoordinateFrame(const CoordinateFrame& cframe);

    void applyTransformation(const LinearTransformation& transformation);
    void applyTransformation(const AffineTransformation& transformation);
};

} // Rendering3D
} // Impact
