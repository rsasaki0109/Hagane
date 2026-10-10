// Shared display-only WebGL viewer. All geometry/normal evaluation lives in Rust.
import {meshDisplayScale, cameraProjection, VERTICAL_FOV} from './view-camera.mjs';
export function createMeshViewer(canvas,{doubleSided=false}={}) {
 const gl=canvas.getContext('webgl',{antialias:true,preserveDrawingBuffer:true});
 if(!gl)throw Error('WebGL is unavailable in this browser.');
 let yaw=0.65,elevation=0.82,zoom=1,scale=1,radius=1,count=0,dragging=false,last=[0,0],wire=false,spin=false,markerCount=0,rawOverlay=null,overlayVisible=true;
 function camera(){return cameraProjection(radius,canvas.clientWidth/Math.max(1,canvas.clientHeight),zoom);}
 function shader(type,source){const s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(s));return s;}
 const p=gl.createProgram();gl.attachShader(p,shader(gl.VERTEX_SHADER,`
attribute vec3 position;attribute vec3 normal;uniform vec3 right;uniform vec3 up;uniform vec3 back;uniform float distance;uniform mat4 projection;varying vec3 n;
void main(){n=normal;vec3 q=vec3(dot(position,right),dot(position,up),dot(position,back)-distance);gl_Position=projection*vec4(q,1.0);gl_PointSize=7.0;}`));
 gl.attachShader(p,shader(gl.FRAGMENT_SHADER,`
precision highp float;varying vec3 n;uniform bool lines;uniform bool doubleSided;uniform vec3 lineColor;
void main(){if(lines){gl_FragColor=vec4(lineColor,1.0);return;}vec3 N=normalize(n);if(doubleSided&&!gl_FrontFacing)N=-N;float light=max(dot(N,normalize(vec3(-0.5,-0.7,1.0))),0.0);float rim=max(dot(N,normalize(vec3(0.7,0.5,0.3))),0.0);vec3 color=vec3(0.66,0.73,0.47)*(0.28+0.68*light)+vec3(0.20,0.27,0.25)*rim;gl_FragColor=vec4(color,1.0);}`));gl.linkProgram(p);if(!gl.getProgramParameter(p,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(p));gl.useProgram(p);gl.enable(gl.DEPTH_TEST);if(!doubleSided)gl.enable(gl.CULL_FACE);
 const locations=Object.fromEntries(['right','up','back','distance','projection','lines','doubleSided','lineColor'].map(k=>[k,gl.getUniformLocation(p,k)]));gl.uniform1i(locations.doubleSided,doubleSided);
 const buffers=Object.fromEntries(['positions','normals','linePositions','lineNormals','marker','markerNormals'].map(k=>[k,gl.createBuffer()]));
 function bind(buffer,attribute,data){gl.bindBuffer(gl.ARRAY_BUFFER,buffer);if(data)gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(data),gl.STATIC_DRAW);const loc=gl.getAttribLocation(p,attribute);gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,3,gl.FLOAT,false,0,0);}
 function render(){const dpr=Math.min(devicePixelRatio,2),w=Math.round(canvas.clientWidth*dpr),h=Math.round(canvas.clientHeight*dpr);if(!w||!h)return;if(canvas.width!==w||canvas.height!==h){canvas.width=w;canvas.height=h;}gl.viewport(0,0,w,h);gl.clearColor(0,0,0,0);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);const back=[Math.sin(yaw)*Math.cos(elevation),-Math.cos(yaw)*Math.cos(elevation),Math.sin(elevation)],right=[Math.cos(yaw),Math.sin(yaw),0],up=[-Math.sin(yaw)*Math.sin(elevation),Math.cos(yaw)*Math.sin(elevation),Math.cos(elevation)];gl.uniform3fv(locations.back,back);gl.uniform3fv(locations.right,right);gl.uniform3fv(locations.up,up);const view=camera();zoom=view.zoom;gl.uniform1f(locations.distance,view.distance);const f=1/Math.tan(VERTICAL_FOV/2),{near,far}=view;gl.uniformMatrix4fv(locations.projection,false,new Float32Array([f/(w/h),0,0,0,0,f,0,0,0,0,(far+near)/(near-far),-1,0,0,2*far*near/(near-far),0]));bind(buffers.positions,'position');bind(buffers.normals,'normal');gl.uniform1i(locations.lines,0);gl.enable(gl.POLYGON_OFFSET_FILL);gl.polygonOffset(1,1);gl.drawArrays(gl.TRIANGLES,0,count);gl.disable(gl.POLYGON_OFFSET_FILL);gl.uniform1i(locations.lines,1);if(wire){gl.uniform3fv(locations.lineColor,[0.12,0.17,0.12]);bind(buffers.linePositions,'position');bind(buffers.lineNormals,'normal');gl.drawArrays(gl.LINES,0,count*2);}if(markerCount){gl.depthFunc(gl.LEQUAL);gl.uniform3fv(locations.lineColor,[0.4,0.9,0.85]);bind(buffers.marker,'position');bind(buffers.markerNormals,'normal');gl.drawArrays(gl.LINES,0,markerCount);gl.drawArrays(gl.POINTS,0,1);gl.depthFunc(gl.LESS);}}
 function uploadOverlay(){
  if(!rawOverlay){markerCount=0;overlayVisible=true;return;}
  let positions,normals;
  if(rawOverlay.kind==='point'){
   const point=rawOverlay.point.map(v=>v/scale),end=point.map((v,i)=>v+rawOverlay.normal[i]*radius*0.28);
   positions=[...point,...end];normals=[...rawOverlay.normal,...rawOverlay.normal];
  }else{positions=rawOverlay.positions.map(v=>v/scale);normals=new Array(positions.length).fill(0);}
  // A distant overlay may be outside GPU range even though the CAD result is valid.
  overlayVisible=positions.every(v=>Number.isFinite(Math.fround(v)))&&normals.every(Number.isFinite);
  markerCount=overlayVisible?positions.length/3:0;
  if(markerCount){bind(buffers.marker,'position',positions);bind(buffers.markerNormals,'normal',normals);}
 }
 function setMesh(mesh){const prepared=meshDisplayScale(mesh.positions,mesh.normals),nextCount=prepared.positions.length/3;
  const lines=[],normals=[];for(let i=0;i<nextCount;i+=3){for(const [a,b] of [[0,1],[1,2],[2,0]]){lines.push(...prepared.positions.slice((i+a)*3,(i+a+1)*3),...prepared.positions.slice((i+b)*3,(i+b+1)*3));normals.push(...mesh.normals.slice((i+a)*3,(i+a+1)*3),...mesh.normals.slice((i+b)*3,(i+b+1)*3));}}
  // Validate and prepare every array before replacing the accepted display state.
  scale=prepared.scale;radius=prepared.radius;count=nextCount;
  bind(buffers.positions,'position',prepared.positions);bind(buffers.normals,'normal',mesh.normals);bind(buffers.linePositions,'position',lines);bind(buffers.lineNormals,'normal',normals);uploadOverlay();render();
 }
 function setMarker(point,normal){rawOverlay=point?{kind:'point',point:[...point],normal:[...normal]}:null;uploadOverlay();render();return overlayVisible;}
 function setSegments(segments){rawOverlay={kind:'segments',positions:segments.flat(2)};uploadOverlay();render();return overlayVisible;}
 Object.defineProperty(canvas,'haganeCamera',{get:()=>Object.freeze({...camera(),scale,radius,yaw,elevation,aspect:canvas.clientWidth/Math.max(1,canvas.clientHeight),vertexCount:count,overlayVisible})});
 const api={setMesh,setMarker,setSegments,render,setAngle(a){yaw=a;render();},reset(){yaw=0.65;elevation=0.82;zoom=1;render();},set wireframe(value){wire=Boolean(value);render();},set autoRotate(value){spin=Boolean(value);}};
 canvas.addEventListener('pointerdown',e=>{dragging=true;last=[e.clientX,e.clientY];canvas.setPointerCapture(e.pointerId);});canvas.addEventListener('pointerup',()=>dragging=false);canvas.addEventListener('pointercancel',()=>dragging=false);canvas.addEventListener('pointermove',e=>{if(!dragging)return;yaw+=(e.clientX-last[0])*0.008;elevation=Math.max(-1.35,Math.min(1.35,elevation+(e.clientY-last[1])*0.006));last=[e.clientX,e.clientY];render();});canvas.addEventListener('wheel',e=>{e.preventDefault();zoom=cameraProjection(radius,canvas.clientWidth/Math.max(1,canvas.clientHeight),zoom*Math.exp(Math.max(-10,Math.min(10,e.deltaY*0.001)))).zoom;render();},{passive:false});canvas.addEventListener('keydown',e=>{if(e.key==='ArrowLeft')yaw-=0.1;else if(e.key==='ArrowRight')yaw+=0.1;else if(e.key==='ArrowUp')elevation=Math.min(1.35,elevation+0.1);else if(e.key==='ArrowDown')elevation=Math.max(-1.35,elevation-0.1);else if(e.key==='+')zoom=cameraProjection(radius,canvas.clientWidth/Math.max(1,canvas.clientHeight),zoom*Math.exp(-0.07)).zoom;else if(e.key==='-')zoom=cameraProjection(radius,canvas.clientWidth/Math.max(1,canvas.clientHeight),zoom*Math.exp(0.07)).zoom;else return;e.preventDefault();render();});new ResizeObserver(render).observe(canvas);let previous=performance.now();function frame(now){if(spin){yaw+=(now-previous)*0.00025;render();}previous=now;requestAnimationFrame(frame);}requestAnimationFrame(frame);return api;
}
