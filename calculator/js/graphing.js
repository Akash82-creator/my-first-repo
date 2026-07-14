/**
 * Graphing Module - 2D function plotting
 */

class Graphing {
    constructor(canvas) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.functions = [];
        this.xMin = -10;
        this.xMax = 10;
        this.yMin = -10;
        this.yMax = 10;
        this.width = canvas.width;
        this.height = canvas.height;
        this.gridColor = '#333';
        this.axisColor = '#666';
        this.textColor = '#aaa';
        
        this.setupCanvas();
    }

    setupCanvas() {
        const rect = this.canvas.getBoundingClientRect();
        this.canvas.width = rect.width * window.devicePixelRatio;
        this.canvas.height = rect.height * window.devicePixelRatio;
        this.ctx.scale(window.devicePixelRatio, window.devicePixelRatio);
        this.width = rect.width;
        this.height = rect.height;
    }

    clear() {
        this.ctx.fillStyle = '#0d1b2a';
        this.ctx.fillRect(0, 0, this.width, this.height);
    }

    drawGrid() {
        this.ctx.strokeStyle = this.gridColor;
        this.ctx.lineWidth = 0.5;
        
        const xStep = this.getGridStep(this.xMax - this.xMin);
        const yStep = this.getGridStep(this.yMax - this.yMin);
        
        // Vertical lines
        for (let x = Math.ceil(this.xMin / xStep) * xStep; x <= this.xMax; x += xStep) {
            const screenX = this.toScreenX(x);
            this.ctx.beginPath();
            this.ctx.moveTo(screenX, 0);
            this.ctx.lineTo(screenX, this.height);
            this.ctx.stroke();
        }
        
        // Horizontal lines
        for (let y = Math.ceil(this.yMin / yStep) * yStep; y <= this.yMax; y += yStep) {
            const screenY = this.toScreenY(y);
            this.ctx.beginPath();
            this.ctx.moveTo(0, screenY);
            this.ctx.lineTo(this.width, screenY);
            this.ctx.stroke();
        }
        
        // Draw axes
        this.ctx.strokeStyle = this.axisColor;
        this.ctx.lineWidth = 1.5;
        
        // X-axis
        if (this.yMin <= 0 && this.yMax >= 0) {
            const yAxis = this.toScreenY(0);
            this.ctx.beginPath();
            this.ctx.moveTo(0, yAxis);
            this.ctx.lineTo(this.width, yAxis);
            this.ctx.stroke();
        }
        
        // Y-axis
        if (this.xMin <= 0 && this.xMax >= 0) {
            const xAxis = this.toScreenX(0);
            this.ctx.beginPath();
            this.ctx.moveTo(xAxis, 0);
            this.ctx.lineTo(xAxis, this.height);
            this.ctx.stroke();
        }
        
        // Labels
        this.ctx.fillStyle = this.textColor;
        this.ctx.font = '12px Arial';
        this.ctx.textAlign = 'center';
        
        for (let x = Math.ceil(this.xMin / xStep) * xStep; x <= this.xMax; x += xStep) {
            if (Math.abs(x) < 1e-10) continue;
            const screenX = this.toScreenX(x);
            const yAxis = this.toScreenY(0);
            this.ctx.fillText(x.toFixed(1), screenX, yAxis + 15);
        }
        
        this.ctx.textAlign = 'right';
        for (let y = Math.ceil(this.yMin / yStep) * yStep; y <= this.yMax; y += yStep) {
            if (Math.abs(y) < 1e-10) continue;
            const screenY = this.toScreenY(y);
            const xAxis = this.toScreenX(0);
            this.ctx.fillText(y.toFixed(1), xAxis - 5, screenY + 4);
        }
    }

    getGridStep(range) {
        const targetSteps = 10;
        const roughStep = range / targetSteps;
        const magnitude = Math.pow(10, Math.floor(Math.log10(roughStep)));
        const residual = roughStep / magnitude;
        
        if (residual < 1.5) return magnitude;
        if (residual < 3.5) return 2 * magnitude;
        if (residual < 7.5) return 5 * magnitude;
        return 10 * magnitude;
    }

    toScreenX(x) {
        return ((x - this.xMin) / (this.xMax - this.xMin)) * this.width;
    }

    toScreenY(y) {
        return this.height - ((y - this.yMin) / (this.yMax - this.yMin)) * this.height;
    }

    toGraphX(screenX) {
        return this.xMin + (screenX / this.width) * (this.xMax - this.xMin);
    }

    toGraphY(screenY) {
        return this.yMin + ((this.height - screenY) / this.height) * (this.yMax - this.yMin);
    }

    plotFunction(expr, color = '#00ff88') {
        try {
            const func = this.createFunction(expr);
            this.ctx.strokeStyle = color;
            this.ctx.lineWidth = 2;
            this.ctx.beginPath();
            
            let started = false;
            for (let screenX = 0; screenX < this.width; screenX++) {
                const x = this.toGraphX(screenX);
                try {
                    const y = func(x);
                    if (!isFinite(y)) {
                        started = false;
                        continue;
                    }
                    
                    const screenY = this.toScreenY(y);
                    
                    if (!started) {
                        this.ctx.moveTo(screenX, screenY);
                        started = true;
                    } else {
                        this.ctx.lineTo(screenX, screenY);
                    }
                } catch (e) {
                    started = false;
                }
            }
            
            this.ctx.stroke();
            return true;
        } catch (error) {
            console.error('Error plotting:', error);
            return false;
        }
    }

    createFunction(expr) {
        // Replace common math functions with JS equivalents
        expr = expr.replace(/\^/g, '**');
        expr = expr.replace(/sin/g, 'Math.sin');
        expr = expr.replace(/cos/g, 'Math.cos');
        expr = expr.replace(/tan/g, 'Math.tan');
        expr = expr.replace(/asin/g, 'Math.asin');
        expr = expr.replace(/acos/g, 'Math.acos');
        expr = expr.replace(/atan/g, 'Math.atan');
        expr = expr.replace(/sinh/g, 'Math.sinh');
        expr = expr.replace(/cosh/g, 'Math.cosh');
        expr = expr.replace(/tanh/g, 'Math.tanh');
        expr = expr.replace(/sqrt/g, 'Math.sqrt');
        expr = expr.replace(/cbrt/g, 'Math.cbrt');
        expr = expr.replace(/abs/g, 'Math.abs');
        expr = expr.replace(/log/g, 'Math.log10');
        expr = expr.replace(/ln/g, 'Math.log');
        expr = expr.replace(/exp/g, 'Math.exp');
        expr = expr.replace(/pi/g, 'Math.PI');
        expr = expr.replace(/e(?![x])/g, 'Math.E');
        
        return new Function('x', `return ${expr}`);
    }

    zoom(factor) {
        const xCenter = (this.xMin + this.xMax) / 2;
        const yCenter = (this.yMin + this.yMax) / 2;
        const xRange = (this.xMax - this.xMin) * factor;
        const yRange = (this.yMax - this.yMin) * factor;
        
        this.xMin = xCenter - xRange / 2;
        this.xMax = xCenter + xRange / 2;
        this.yMin = yCenter - yRange / 2;
        this.yMax = yCenter + yRange / 2;
    }

    pan(dx, dy) {
        const dxGraph = this.toGraphX(0) - this.toGraphX(dx);
        const dyGraph = this.toGraphY(0) - this.toGraphY(dy);
        
        this.xMin += dxGraph;
        this.xMax += dxGraph;
        this.yMin += dyGraph;
        this.yMax += dyGraph;
    }

    findIntersections(func1, func2) {
        const intersections = [];
        const step = (this.xMax - this.xMin) / 1000;
        
        for (let x = this.xMin; x < this.xMax; x += step) {
            const y1a = func1(x);
            const y2a = func2(x);
            const y1b = func1(x + step);
            const y2b = func2(x + step);
            
            if ((y1a - y2a) * (y1b - y2b) < 0) {
                // Binary search for intersection
                let lo = x, hi = x + step;
                for (let i = 0; i < 20; i++) {
                    const mid = (lo + hi) / 2;
                    if ((func1(lo) - func2(lo)) * (func1(mid) - func2(mid)) < 0) {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                intersections.push({ x: (lo + hi) / 2, y: func1((lo + hi) / 2) });
            }
        }
        
        return intersections;
    }

    render() {
        this.clear();
        this.drawGrid();
        
        for (const fn of this.functions) {
            this.plotFunction(fn.expr, fn.color);
        }
    }

    addFunction(expr, color) {
        this.functions.push({ expr, color });
        this.render();
    }

    clearFunctions() {
        this.functions = [];
        this.render();
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = Graphing;
}
