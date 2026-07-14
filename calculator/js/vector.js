/**
 * Vector Calculator - 2D and 3D vector operations
 */

class Vector {
    constructor(...components) {
        this.components = components;
        this.dimension = components.length;
    }

    static fromArray(arr) {
        return new Vector(...arr);
    }

    get(index) {
        return this.components[index];
    }

    set(index, value) {
        this.components[index] = value;
    }

    add(other) {
        if (this.dimension !== other.dimension) {
            throw new Error('Vectors must have the same dimension');
        }
        return new Vector(...this.components.map((c, i) => c + other.components[i]));
    }

    subtract(other) {
        if (this.dimension !== other.dimension) {
            throw new Error('Vectors must have the same dimension');
        }
        return new Vector(...this.components.map((c, i) => c - other.components[i]));
    }

    multiply(scalar) {
        return new Vector(...this.components.map(c => c * scalar));
    }

    divide(scalar) {
        if (scalar === 0) throw new Error('Division by zero');
        return new Vector(...this.components.map(c => c / scalar));
    }

    dot(other) {
        if (this.dimension !== other.dimension) {
            throw new Error('Vectors must have the same dimension');
        }
        return this.components.reduce((sum, c, i) => sum + c * other.components[i], 0);
    }

    cross(other) {
        if (this.dimension !== 3 || other.dimension !== 3) {
            throw new Error('Cross product only defined for 3D vectors');
        }
        const x = this.components[1] * other.components[2] - this.components[2] * other.components[1];
        const y = this.components[2] * other.components[0] - this.components[0] * other.components[2];
        const z = this.components[0] * other.components[1] - this.components[1] * other.components[0];
        return new Vector(x, y, z);
    }

    magnitude() {
        return Math.sqrt(this.components.reduce((sum, c) => sum + c * c, 0));
    }

    normalize() {
        const mag = this.magnitude();
        if (mag === 0) throw new Error('Cannot normalize zero vector');
        return this.divide(mag);
    }

    angle(other) {
        const dotProduct = this.dot(other);
        const magProduct = this.magnitude() * other.magnitude();
        if (magProduct === 0) throw new Error('Cannot compute angle with zero vector');
        const cosAngle = dotProduct / magProduct;
        // Clamp to [-1, 1] to handle floating point errors
        const clamped = Math.max(-1, Math.min(1, cosAngle));
        return Math.acos(clamped);
    }

    angleDegrees(other) {
        return this.angle(other) * 180 / Math.PI;
    }

    projectOnto(other) {
        const dotProduct = this.dot(other);
        const magSquared = other.magnitude() ** 2;
        if (magSquared === 0) throw new Error('Cannot project onto zero vector');
        return other.multiply(dotProduct / magSquared);
    }

    rejectFrom(other) {
        const projection = this.projectOnto(other);
        return this.subtract(projection);
    }

    distanceTo(other) {
        return this.subtract(other).magnitude();
    }

    toArray() {
        return [...this.components];
    }

    toString(precision = 4) {
        return `<${this.components.map(c => c.toFixed(precision)).join(', ')}>`;
    }

    // Static factory methods
    static zero(dim = 3) {
        return new Vector(...Array(dim).fill(0));
    }

    static unitX() { return new Vector(1, 0, 0); }
    static unitY() { return new Vector(0, 1, 0); }
    static unitZ() { return new Vector(0, 0, 1); }

    // Create from spherical coordinates (r, θ, φ)
    static fromSpherical(r, theta, phi) {
        const x = r * Math.sin(theta) * Math.cos(phi);
        const y = r * Math.sin(theta) * Math.sin(phi);
        const z = r * Math.cos(theta);
        return new Vector(x, y, z);
    }

    // Create from cylindrical coordinates (ρ, φ, z)
    static fromCylindrical(rho, phi, z) {
        const x = rho * Math.cos(phi);
        const y = rho * Math.sin(phi);
        return new Vector(x, y, z);
    }

    // Convert to spherical coordinates
    toSpherical() {
        const r = this.magnitude();
        const theta = Math.acos(this.components[2] / (r || 1));
        const phi = Math.atan2(this.components[1], this.components[0]);
        return { r, theta, phi };
    }

    // Convert to cylindrical coordinates
    toCylindrical() {
        const rho = Math.sqrt(this.components[0] ** 2 + this.components[1] ** 2);
        const phi = Math.atan2(this.components[1], this.components[0]);
        const z = this.components[2];
        return { rho, phi, z };
    }
}

// Vector utility functions
class VectorUtils {
    static areParallel(v1, v2) {
        const cross = v1.cross(v2);
        return cross.magnitude() < 1e-10;
    }

    static arePerpendicular(v1, v2) {
        return Math.abs(v1.dot(v2)) < 1e-10;
    }

    static tripleScalarProduct(a, b, c) {
        return a.dot(b.cross(c));
    }

    static tripleVectorProduct(a, b, c) {
        // a × (b × c) = b(a · c) - c(a · b)
        return b.multiply(a.dot(c)).subtract(c.multiply(a.dot(b)));
    }

    static gramSchmidt(vectors) {
        const orthogonal = [];
        for (const v of vectors) {
            let u = v;
            for (const e of orthogonal) {
                u = u.subtract(v.projectOnto(e));
            }
            if (u.magnitude() > 1e-10) {
                orthogonal.push(u.normalize());
            }
        }
        return orthogonal;
    }

    static centroid(vectors) {
        if (vectors.length === 0) throw new Error('Cannot compute centroid of empty set');
        const dim = vectors[0].dimension;
        const sum = Array(dim).fill(0);
        for (const v of vectors) {
            for (let i = 0; i < dim; i++) {
                sum[i] += v.components[i];
            }
        }
        return new Vector(...sum.map(s => s / vectors.length));
    }

    static coplanar(v1, v2, v3, v4) {
        // Four points are coplanar if the volume of the parallelepiped is zero
        const a = v2.subtract(v1);
        const b = v3.subtract(v1);
        const c = v4.subtract(v1);
        return Math.abs(VectorUtils.tripleScalarProduct(a, b, c)) < 1e-10;
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = { Vector, VectorUtils };
}
