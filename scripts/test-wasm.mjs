import {readFileSync,writeFileSync,unlinkSync} from 'node:fs';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
const {instance}=await WebAssembly.instantiate(readFileSync(new URL('../web/hagane.wasm',import.meta.url)),{});
const k=instance.exports;
function generate(radius,error=0.05){const status=k.hagane_generate(radius,error);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const radius of [8,14,24,8]){const {status,result}=generate(radius);assert.equal(status,0);assert.ok(Math.abs(result.volume-(80*60*24-Math.PI*radius**2*24))<1e-8);assert.equal(result.faces,7);assert.equal(result.edges,15);assert.ok(result.positions.length>0);assert.equal(result.positions.length%9,0);assert.equal(result.normals.length,result.positions.length);assert.ok(result.positions.every(Number.isFinite));}
for(const radius of [30,0,-1,NaN,Infinity]){const {status,result}=generate(radius);assert.equal(status,1);assert.ok(result.error);}
assert.equal(generate(14,0).status,1);
assert.equal(generate(14).status,0);
console.log('WASM runtime: 11 generation/error/recovery checks passed.');
function preset(id,radius=14,error=0.05){const status=k.hagane_generate_preset(id,radius,error);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const control of [8,14,24]){
 const {status,result}=preset(25,control);assert.equal(status,0);
 assert.ok(Math.abs(result.volume-Math.PI*(576-(control/2)**2/Math.cos(.5))*16)<1e-8);
 assert.equal(result.faces,6);assert.equal(result.edges,12);
}
for(const control of [0,7,25,NaN,Infinity])assert.equal(preset(25,control).status,1);
assert.equal(preset(25,14,0).status,1);assert.equal(preset(25).status,0);
const expected=[{volume:115200-Math.PI*196*24,faces:7,edges:15},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:69336,faces:12,edges:30},{volume:Math.PI*(900-196)*24,faces:4,edges:6},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:(4800-(4-Math.PI)*196)*24,faces:10,edges:24},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42},{volume:115200-Math.PI*49*24,faces:8,edges:18},{volume:(4800-(4-Math.PI)*576)*24,faces:13,edges:31},{volume:103680,faces:15,edges:45},{volume:5376*Math.PI,faces:11,edges:26},{volume:115200-3456*Math.PI,faces:13,edges:34},{volume:115200,faces:6,edges:13},{volume:67200,faces:6,edges:12},{volume:(4096-(32*Math.SQRT2-38)**2-(32*Math.SQRT2-42)**2-2*(32*Math.SQRT2-30)**2)*24,faces:10,edges:24},{volume:107520,faces:20,edges:44},{volume:174720,faces:24,edges:52},{volume:122880,faces:26,edges:52},{volume:122880,faces:10,edges:34},{volume:174720,faces:12,edges:38},{volume:122880,faces:10,edges:24},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42},{volume:(64*48-(4-Math.PI)*100-(20*16-(4-Math.PI)*16))*24,faces:22,edges:58},{volume:(64*48-(4-Math.PI)*100-(20*16-(4-Math.PI)*16))*24,faces:34,edges:80},{volume:Math.PI*(576-49/Math.cos(.5))*16,faces:6,edges:12},{volume:Math.PI*(576-2*(14/6)**2/Math.cos(1))*8,faces:8,edges:18},{volume:Math.PI*(576-2*(14/6)**2/Math.cos(.7))*8,faces:8,edges:18},{volume:Math.PI*(784-(14/6)**2*(1/Math.cos(.7)+1/Math.cos(.5)))*8,faces:8,edges:18},{volume:115200-Math.PI*196*16,faces:8,edges:15},{volume:192000-Math.PI*49*20,faces:8,edges:15}];
for(let id=0;id<expected.length;id++){
 const {status,result:wasm}=preset(id);assert.equal(status,0);assert.ok(Math.abs(wasm.volume-expected[id].volume)<1e-8);assert.equal(wasm.faces,expected[id].faces);assert.equal(wasm.edges,expected[id].edges);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','part','--',String(id)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
 assert.ok(Math.abs(native.volume-wasm.volume)<1e-9);
 for(const field of ['positions','normals']){assert.equal(native[field].length,wasm[field].length);native[field].forEach((v,i)=>assert.ok(Math.abs(v-wasm[field][i])<1e-10));}
}
assert.equal(preset(4,30).status,1);assert.equal(preset(4,NaN).status,1);assert.equal(preset(4).status,0);
for(const radius of [8,14,24]){const {status,result}=preset(5,radius);assert.equal(status,0);assert.ok(Math.abs(result.volume-(4800-(4-Math.PI)*radius**2)*24)<1e-8);}
for(const radius of [0,-1,30,NaN,Infinity])assert.equal(preset(5,radius).status,1);
assert.equal(preset(5,14,0).status,1);assert.equal(preset(5).status,0);
for(const radius of [8,14,24]){const {status,result}=preset(6,radius);assert.equal(status,0);assert.ok(Math.abs(result.volume-(4600+(4-Math.PI)*9-Math.PI*radius**2/2)*24)<1e-8);}
for(const radius of [0,-1,30,NaN,Infinity])assert.equal(preset(6,radius).status,1);
assert.equal(preset(6,14,0).status,1);assert.equal(preset(6).status,0);
assert.equal(preset(999).status,1);assert.equal(preset(1,30).status,1);assert.equal(preset(3,30).status,1);assert.equal(preset(2,14,0).status,1);assert.equal(preset(1).status,0);
console.log('All 31 presets: native/WASM geometry parity, metrics, errors and recovery passed.');
function nurbs(weight,parameter){const status=k.hagane_generate_nurbs(weight,parameter);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const weight of [Math.SQRT1_2,0.1,1,2]){for(const parameter of [0,0.25,0.5,1]){
 const {status,result}=nurbs(weight,parameter);assert.equal(status,0);assert.equal(result.samples.length,129*3);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs','--',String(weight),String(parameter)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
 for(const field of ['samples','point','tangent']){assert.equal(native[field].length,result[field].length);native[field].forEach((value,i)=>assert.ok(Math.abs(value-result[field][i])<1e-12));}
 if(weight===Math.SQRT1_2){for(let i=0;i<result.samples.length;i+=3){assert.ok(Math.abs(result.samples[i]**2+result.samples[i+1]**2-1)<1e-14);}assert.ok(Math.abs(result.point[0]*result.tangent[0]+result.point[1]*result.tangent[1])<1e-13);}
}}
for(const [weight,parameter] of [[0,0.5],[-1,0.5],[NaN,0.5],[1,NaN],[1,1.001],[1,-0.001]]){assert.equal(nurbs(weight,parameter).status,1);}
assert.equal(nurbs(Math.SQRT1_2,0.5).status,0);
console.log('NURBS: 16 native/WASM parity cases, exact quarter-circle samples/tangents, invalid input and recovery passed.');
function boundedNurbs(weight,parameter,error){const status=k.hagane_generate_nurbs_bounded(weight,parameter,error);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
// Independent rational Bernstein evaluation of the original, unrefined curve.
function rationalPoint(weight,u){const a=(1-u)**2,b=2*u*(1-u)*weight,c=u*u,d=a+b+c;return [(a+b)/d,(b+c)/d,0];}
function pointChordDistance(p,a,b){const d=b.map((v,i)=>v-a[i]),length2=d.reduce((s,v)=>s+v*v,0),t=length2?Math.max(0,Math.min(1,d.reduce((s,v,i)=>s+v*(p[i]-a[i]),0)/length2)):0;return Math.hypot(...p.map((v,i)=>v-a[i]-t*d[i]));}
let boundedCases=0;
for(const weight of [0.1,Math.SQRT1_2,1,2])for(const error of [0.01,0.001,0.0001]){
 const {status,result}=boundedNurbs(weight,0.5,error);assert.equal(status,0,JSON.stringify(result));
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs_refinement','--',String(weight),'0.5',String(error)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 assert.equal(result.control_count,6);assert.deepEqual(result.inserted_knots,[0.35,0.7,0.7]);assert.deepEqual(result.span_ranges,[[0,0.35],[0.35,0.7],[0.7,1]]);assert.equal(result.refined_controls.length,18);assert.equal(result.refined_weights.length,6);
 assert.equal(result.samples.length,(result.segments+1)*3);assert.equal(result.parameters.length,result.segments+1);assert.equal(result.error_bounds.length,result.segments);assert.equal(result.requested_error,error);assert.equal(result.parameters[0],0);assert.equal(result.parameters.at(-1),1);
 for(const knot of [0.35,0.7])assert.ok(result.parameters.includes(knot));
 for(let i=0;i<result.parameters.length;i++){const point=result.samples.slice(3*i,3*i+3),expected=rationalPoint(weight,result.parameters[i]);expected.forEach((v,j)=>assert.ok(Math.abs(point[j]-v)<1e-13));}
 for(let i=0;i<result.segments;i++){
  const [start,end]=[result.parameters[i],result.parameters[i+1]],bound=result.error_bounds[i],a=result.samples.slice(3*i,3*i+3),b=result.samples.slice(3*i+3,3*i+6);
  assert.ok(end>start);assert.ok(Number.isFinite(bound)&&bound>0&&bound<=error);
  for(let j=0;j<=16;j++)assert.ok(pointChordDistance(rationalPoint(weight,start+(end-start)*j/16),a,b)<=bound+1e-14,`weight=${weight}, error=${error}, interval=${i}`);
 }
 boundedCases++;
}
for(const args of [[0,0.5,0.001],[-1,0.5,0.001],[NaN,0.5,0.001],[Infinity,0.5,0.001],[1,-0.01,0.001],[1,1.01,0.001],[1,NaN,0.001],[1,Infinity,0.001],[1,0.5,0],[1,0.5,-1],[1,0.5,NaN],[1,0.5,Infinity],[1,0.5,1e-14]]){const {status,result}=boundedNurbs(...args);assert.equal(status,1);assert.equal(typeof result.error,'string');}
assert.equal(boundedNurbs(Math.SQRT1_2,0.5,0.001).status,0);
console.log(`Bounded NURBS: ${boundedCases} refined-curve native/WASM cases, independent interval chord errors, knot preservation, invalid input and recovery passed.`);
function surface(height,weight,u,v){const status=k.hagane_generate_surface(height,weight,u,v);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
// Tensor Bernstein oracle uses the original 3x3 patch, before any knot insertion.
function surfacePoint(height,weight,u,v){const bu=[(1-u)**2,2*u*(1-u),u*u],bv=[(1-v)**2,2*v*(1-v),v*v];let denominator=0;const p=[0,0,0];for(let i=0;i<3;i++)for(let j=0;j<3;j++){const center=i===1&&j===1,w=bu[i]*bv[j]*(center?weight:1),control=[-40+40*i,-30+30*j,center?height:0];denominator+=w;for(let k=0;k<3;k++)p[k]+=w*control[k];}return p.map(x=>x/denominator);}
function sectionPoint(section,t){const n=section.control_points.length,knots=section.knots;function basis(i,d){if(d===0)return t===knots.at(-1)?Number(i===n-1):Number(knots[i]<=t&&t<knots[i+1]);const left=knots[i+d]-knots[i],right=knots[i+d+1]-knots[i+1];return (left?(t-knots[i])/left*basis(i,d-1):0)+(right?(knots[i+d+1]-t)/right*basis(i+1,d-1):0);}let denominator=0;const p=[0,0,0];for(let i=0;i<n;i++){const b=basis(i,section.degree)*section.weights[i];denominator+=b;for(let j=0;j<3;j++)p[j]+=b*section.control_points[i][j];}return p.map(x=>x/denominator);}
for(const [height,weight,u,v] of [[0,1,0.5,0.5],[35,1,0.5,0.5],[35,2,0.3,0.7],[-35,0.5,0.2,0.4],[60,4,0,1],[-60,0.2,1,0]]){
 const {status,result}=surface(height,weight,u,v);assert.equal(status,0);assert.equal(result.positions.length,24*24*2*9);assert.equal(result.positions.length,result.normals.length);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs_surface','--',...[height,weight,u,v].map(String)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
 for(const field of ['point','du','dv','normal','positions','normals']){assert.equal(result[field].length,native[field].length);native[field].forEach((value,i)=>assert.ok(Math.abs(value-result[field][i])<1e-10));}
 for(const field of ['sections','section_chord_error','refined_control_counts','brep'])compareIntersection(result[field],native[field]);
 const topology=result.brep;assert.equal(topology.faces,1);assert.equal(topology.closed,false);assert.equal(topology.orientation,1);assert.equal(topology.validation,'canonical rectangular boundary; global surface regularity is not certified');assert.equal(topology.vertices.length,4);
 assert.deepEqual(topology.edge_vertices,[[0,1],[1,2],[3,2],[0,3]]);assert.deepEqual(topology.edge_ranges,[[0,1],[0,1],[0,1],[0,1]]);assert.deepEqual(topology.coedge_edges,[0,1,2,3]);assert.deepEqual(topology.coedge_forward,[true,true,false,false]);
 [[0,0],[1,0],[1,1],[0,1]].forEach(([a,b],i)=>compareIntersection(topology.vertices[i],surfacePoint(height,weight,a,b)));
 const orientedEdges=topology.coedge_edges.map((edge,i)=>topology.coedge_forward[i]?topology.edge_vertices[edge]:topology.edge_vertices[edge].toReversed());orientedEdges.forEach((edge,i)=>assert.equal(edge[1],orientedEdges[(i+1)%4][0]));
 assert.deepEqual(result.refined_control_counts,[4,4]);assert.equal(result.section_chord_error,0.05);assert.equal(result.sections.length,6);
 const configs=[[1,0,true,true],[0,1,true,true],[1,1,true,false],[0,0,true,false],[0,u,false,true],[1,v,false,true]];
 for(let s=0;s<6;s++){
  const section=result.sections[s];assert.deepEqual(section.pcurve_origin,section.fixed_axis===0?[section.fixed_parameter,0]:[0,section.fixed_parameter]);assert.deepEqual(section.pcurve_direction,section.fixed_axis===0?[0,1]:[1,0]);assert.deepEqual([section.fixed_axis,section.fixed_parameter,section.boundary,section.forward],configs[s]);assert.deepEqual(section.parameter_range,[0,1]);assert.equal(section.degree,2);assert.equal(section.control_points.length,4);assert.equal(section.weights.length,4);assert.equal(section.knots.length,7);assert.ok(section.weights.every(w=>w>0&&Number.isFinite(w)));
  assert.equal(section.parameters[0],0);assert.equal(section.parameters.at(-1),1);assert.equal(section.samples.length,section.parameters.length*3);assert.equal(section.error_bounds.length,section.parameters.length-1);
  const exact=t=>surfacePoint(height,weight,section.fixed_axis===0?section.fixed_parameter:t,section.fixed_axis===1?section.fixed_parameter:t);
  for(let i=0;i<section.parameters.length;i++)compareIntersection(section.samples.slice(i*3,i*3+3),exact(section.parameters[i]));
  for(let i=0;i<section.error_bounds.length;i++){
   const start=section.parameters[i],end=section.parameters[i+1],bound=section.error_bounds[i],a=section.samples.slice(i*3,i*3+3),b=section.samples.slice(i*3+3,i*3+6);assert.ok(end>start);assert.ok(bound>0&&bound<=result.section_chord_error);
   for(let j=0;j<=16;j++){const t=start+(end-start)*j/16;assert.ok(pointChordDistance(exact(t),a,b)<=bound+1e-10);compareIntersection(sectionPoint(section,t),exact(t));}
  }
  if(!section.boundary)compareIntersection(sectionPoint(section,section.fixed_axis===0?v:u),result.point);
 }
 for(let i=0;i<4;i++){const section=result.sections[i],next=result.sections[(i+1)%4];compareIntersection(section.forward?section.samples.slice(-3):section.samples.slice(0,3),next.forward?next.samples.slice(0,3):next.samples.slice(-3));}
 assert.ok(Math.abs(Math.hypot(...result.normal)-1)<1e-13);const dot=(a,b)=>a.reduce((s,x,i)=>s+x*b[i],0);assert.ok(Math.abs(dot(result.du,result.normal))<1e-10);assert.ok(Math.abs(dot(result.dv,result.normal))<1e-10);
 if(height===0){assert.ok(Math.abs(result.point[0]-(80*u-40))<1e-12);assert.ok(Math.abs(result.point[1]-(60*v-30))<1e-12);assert.equal(result.point[2],0);assert.deepEqual(result.normal,[0,0,1]);}
 if(height===35&&weight===1&&u===0.5){assert.ok(Math.abs(result.point[2]-8.75)<1e-13);}
}
for(const args of [[0,0,0.5,0.5],[101,1,0.5,0.5],[NaN,1,0.5,0.5],[0,1,1.1,0.5],[0,1,0.5,-0.1]]){assert.equal(surface(...args).status,1);}
assert.equal(surface(35,1,0.5,0.5).status,0);
console.log('NURBS surfaces: 6 native/WASM grid/partial/normal/exact-section cases, independent tensor evaluation, interval chord bounds, oriented boundary closure and errors/recovery passed.');
function boundedSurface(height,weight,u,v,error){const status=k.hagane_generate_surface_bounded(height,weight,u,v,error);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
function checkSurfaceMesh(data,oracle=surfacePoint){
 const domain=data.domain??[[0,1],[0,1]];
 assert.equal(data.triangles,2*data.cells);assert.equal(data.positions.length,data.cells*18);assert.equal(data.normals.length,data.positions.length);assert.equal(data.cell_bounds.length,data.cells);assert.equal(data.cell_ranges.length,data.cells);
 const patches=data.patch_ranges??[[[0,1],[0,1]]],grid=new Map(),rectangles=new Set(),side=Math.sqrt(data.cells/patches.length),counts=patches.map(()=>0);assert.ok(Number.isInteger(side));let area=0;
 for(let i=0;i<data.cells;i++){
  const [[u0,u1],[v0,v1]]=data.cell_ranges[i],bound=data.cell_bounds[i];assert.ok(domain[0][0]<=u0&&u0<u1&&u1<=domain[0][1]&&domain[1][0]<=v0&&v0<v1&&v1<=domain[1][1]);const index=patches.findIndex(([[a,b],[c,d]])=>u0>=a&&u1<=b&&v0>=c&&v1<=d);assert.ok(index>=0);counts[index]++;const [[a,b],[c,d]]=patches[index];assert.ok(Math.abs((u1-u0)-(b-a)/side)<1e-14&&Math.abs((v1-v0)-(d-c)/side)<1e-14);for(const x of [(u0-a)*side/(b-a),(v0-c)*side/(d-c)])assert.ok(Math.abs(x-Math.round(x))<1e-10);assert.ok(Number.isFinite(bound)&&bound>0&&bound<=data.surface_error);const rect=JSON.stringify(data.cell_ranges[i]);assert.ok(!rectangles.has(rect));rectangles.add(rect);area+=(u1-u0)*(v1-v0);
  const uv=[[u0,v0],[u1,v0],[u1,v1],[u0,v0],[u1,v1],[u0,v1]],p=uv.map((_,j)=>data.positions.slice(i*18+j*3,i*18+j*3+3));
  uv.forEach(([u,v],j)=>{compareIntersection(p[j],oracle(data.height,data.weight,u,v));const key=`${u},${v}`;if(grid.has(key))assert.deepEqual(p[j],grid.get(key));else grid.set(key,p[j]);const normal=data.normals.slice(i*18+j*3,i*18+j*3+3);assert.ok(Math.abs(Math.hypot(...normal)-1)<1e-12);assert.ok(normal[2]>0);});
  for(let triangle=0;triangle<2;triangle++){const [a,b,c]=p.slice(triangle*3,triangle*3+3),ab=b.map((x,j)=>x-a[j]),ac=c.map((x,j)=>x-a[j]);assert.ok(ab[0]*ac[1]-ab[1]*ac[0]>0);}
  for(let a=0;a<=4;a++)for(let b=0;b<=4;b++){const s=a/4,t=b/4,exact=oracle(data.height,data.weight,u0+(u1-u0)*s,v0+(v1-v0)*t),linear=s>=t?p[0].map((x,j)=>(1-s)*x+(s-t)*p[1][j]+t*p[2][j]):p[0].map((x,j)=>(1-t)*x+s*p[2][j]+(t-s)*p[5][j]);assert.ok(Math.hypot(...exact.map((x,j)=>x-linear[j]))<=bound+1e-10,`surface cell=${i}, s=${s}, t=${t}`);}
 }
 assert.ok(Math.abs(area-(domain[0][1]-domain[0][0])*(domain[1][1]-domain[1][0]))<1e-10);const nu=new Set(patches.flatMap(p=>p[0])).size-1,nv=new Set(patches.flatMap(p=>p[1])).size-1;assert.equal(grid.size,(nu*side+1)*(nv*side+1));assert.equal(rectangles.size,data.cells);counts.forEach(n=>assert.equal(n,side**2));
}
for(const args of [[0,1,0.5,0.5,0.05],[35,1,0.3,0.7,0.1],[-35,0.5,0.2,0.4,0.1],[60,4,0,1,0.1]]){
 const {status,result}=boundedSurface(...args);assert.equal(status,0,JSON.stringify(result));const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs_surface_bounded','--',...args.map(String)],{encoding:'utf8',maxBuffer:32*1024*1024,cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);checkSurfaceMesh(result);
}
for(const error of [0,-1,NaN,Infinity,1e-14]){const {status,result}=boundedSurface(35,1,0.5,0.5,error);assert.equal(status,1);assert.equal(typeof result.error,'string');}
assert.equal(boundedSurface(35,1,0.5,0.5,0.05).status,0);
console.log('Bounded surfaces: 4 native/WASM mesh cases, independent tensor-to-triangle bounds, UV coverage, shared grid vertices, orientation and errors/recovery passed.');
function multispanSurface(height,weight,u,v,error){const status=k.hagane_generate_surface_multispan(height,weight,u,v,error);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
const fourPatches=[[[0,0.35],[0,0.7]],[[0,0.35],[0.7,1]],[[0.35,1],[0,0.7]],[[0.35,1],[0.7,1]]];
for(const args of [[0,1,0.5,0.5,0.1],[35,1,0.3,0.7,0.1],[-35,0.5,0.2,0.4,0.1],[60,4,0,1,0.1]]){
 const {status,result}=multispanSurface(...args);assert.equal(status,0,JSON.stringify(result));const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs_surface_multispan','--',...args.map(String)],{encoding:'utf8',maxBuffer:32*1024*1024,cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);assert.equal(result.refined,true);assert.deepEqual(result.control_counts,[4,4]);assert.deepEqual(result.patch_ranges,fourPatches);checkSurfaceMesh(result);
 const {status:sourceStatus,result:source}=boundedSurface(...args);assert.equal(sourceStatus,0);assert.equal(source.refined,false);assert.deepEqual(source.control_counts,[3,3]);assert.deepEqual(source.patch_ranges,[[[0,1],[0,1]]]);for(const field of ['point','du','dv','normal'])compareIntersection(result[field],source[field]);compareIntersection(result.brep.vertices,source.brep.vertices);
}
for(const error of [0,NaN,Infinity,1e-14])assert.equal(multispanSurface(35,1,0.5,0.5,error).status,1);
assert.equal(multispanSurface(35,1,0.5,0.5,0.1).status,0);
const exhaustedSurface=multispanSurface(60,4,0.5,0.5,0.02);assert.equal(exhaustedSurface.status,1);assert.match(exhaustedSurface.result.error,/cell|depth|limit/i);assert.equal(multispanSurface(60,4,0.5,0.5,0.1).status,0);
console.log('Multi-span surfaces: 4 native/WASM refined shape cases, independent triangle bounds, per-patch UV coverage, identical shared seam vertices, original shape preservation and errors/recovery passed.');
function creaseSurface(...args){const status=k.hagane_generate_surface_crease(...args);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
function roofPoint(height,weight,u,v){if(u<=0.5){const s=2*u,f=weight*s/(1-s+weight*s);return [-40+40*f,60*v-30,height*f];}const s=2*u-1,f=s/(weight*(1-s)+s);return [40*f,60*v-30,height*(1-f)];}
function roofNormal(height,right){const n=[(right?1:-1)*height/40,0,1],length=Math.hypot(...n);return n.map(x=>x/length);}
function checkCreaseTopology(data){
 checkSurfaceMesh(data,roofPoint);assert.equal(data.crease,true);assert.deepEqual(data.control_counts,[3,2]);assert.deepEqual(data.patch_ranges,[[[0,0.5],[0,1]],[[0.5,1],[0,1]]]);assert.equal(data.triangle_nodes.length,data.triangles);assert.equal(data.triangle_uvs.length,data.triangles);
 compareIntersection(data.point,roofPoint(data.height,data.weight,data.u,data.v));compareIntersection(data.normal,roofNormal(data.height,data.u===0.5?data.normal_side===1:data.u>0.5));
 const nodes=new Map(),edges=new Map(),uvNodes=new Map(),seam=new Map();
 data.triangle_nodes.forEach((ids,t)=>{
  assert.equal(new Set(ids).size,3);const right=data.cell_ranges[Math.floor(t/2)][0][0]>=0.5;
  ids.forEach((id,j)=>{assert.ok(Number.isInteger(id)&&id>=0);const p=data.positions.slice(t*9+j*3,t*9+j*3+3),uv=data.triangle_uvs[t][j],normal=data.normals.slice(t*9+j*3,t*9+j*3+3);compareIntersection(p,roofPoint(data.height,data.weight,...uv));compareIntersection(normal,roofNormal(data.height,right));if(nodes.has(id))assert.deepEqual(nodes.get(id),{p,uv});else nodes.set(id,{p,uv});const key=JSON.stringify(uv);if(uvNodes.has(key))assert.equal(uvNodes.get(key),id);else uvNodes.set(key,id);if(uv[0]===0.5){if(!seam.has(key))seam.set(key,new Set());seam.get(key).add(right);}});
  for(let j=0;j<3;j++){const a=ids[j],b=ids[(j+1)%3],key=[Math.min(a,b),Math.max(a,b)].join(',');const edge=edges.get(key)??{a,b,count:0,balance:0};edge.count++;edge.balance+=a<b?1:-1;edges.set(key,edge);}
 });
 assert.equal(nodes.size,data.geometric_vertices);const side=Math.sqrt(data.cells/2);assert.equal(nodes.size,(2*side+1)*(side+1));assert.ok(data.shading_vertices>=data.geometric_vertices);if(data.height!==0)assert.ok(data.shading_vertices>data.geometric_vertices);
 for(const edge of edges.values()){const a=nodes.get(edge.a).uv,b=nodes.get(edge.b).uv,outer=[0,1].some(i=>a[i]===b[i]&&(a[i]===0||a[i]===1));assert.equal(edge.count,outer?1:2);assert.equal(Math.abs(edge.balance),outer?1:0);}
 assert.equal(seam.size,side+1);for(const sides of seam.values())assert.deepEqual([...sides].sort(),[false,true]);
 for(const section of data.sections){const exact=t=>roofPoint(data.height,data.weight,section.fixed_axis===0?section.fixed_parameter:t,section.fixed_axis===1?section.fixed_parameter:t);section.parameters.forEach((t,i)=>compareIntersection(section.samples.slice(i*3,i*3+3),exact(t)));}
}
for(const args of [[35,1,0.5,0.5,0.05,0],[35,1,0.5,0.5,0.05,1],[-35,2,0.5,0.7,0.5,1],[0,1,0.75,0.2,0.05,1]]){
 const {status,result}=creaseSurface(...args);assert.equal(status,0,JSON.stringify(result));const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs_surface_crease','--',...args.map(String)],{encoding:'utf8',maxBuffer:32*1024*1024,cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);checkCreaseTopology(result);
}
for(const args of [[35,1,0.5,0.5,0.05,2],[35,0,0.5,0.5,0.05,0],[35,-1,0.5,0.5,0.05,0],[35,NaN,0.5,0.5,0.05,0],[NaN,1,0.5,0.5,0.05,0],[35,1,NaN,0.5,0.05,0],[35,1,0.5,0.5,0,0],[35,1,0.5,0.5,NaN,0],[35,1,0.5,0.5,1e-14,0]])assert.equal(creaseSurface(...args).status,1);
assert.equal(creaseSurface(35,1,0.5,0.5,0.05,0).status,0);
console.log('C0 roof: 4 native/WASM cases, independent rational geometry/error bounds, one-sided facet normals, welded geometric seam topology, oriented edge incidence and errors/recovery passed.');
function trimmedSurface(...args){const status=k.hagane_generate_surface_trim(...args);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const args of [[35,1,0.5,0.5,0.1,0.2,0.8,0.15,0.85,0],[-35,2,0.55,0.4,0.5,0.35,0.75,0.1,0.7,0],[35,1,0.5,0.5,0.1,0.2,0.8,0.1,0.9,1],[-35,2,0.5,0.45,0.5,0.5,0.8,0.2,0.7,1],[35,1,0.5,0.5,0.1,0.2,0.5,0.1,0.9,1]]){
 const {status,result}=trimmedSurface(...args);assert.equal(status,0,JSON.stringify(result));const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs_surface_trim','--',...args.map(String)],{encoding:'utf8',maxBuffer:32*1024*1024,cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);const oracle=result.crease?roofPoint:surfacePoint;checkSurfaceMesh(result,oracle);
 const ranges=[[args[5],args[6]],[args[7],args[8]]];assert.deepEqual(result.domain,ranges);assert.deepEqual(result.trim_ranges,ranges);assert.deepEqual(result.source_domain,[[0,1],[0,1]]);compareIntersection(result.point,oracle(args[0],args[1],args[2],args[3]));assert.equal(result.brep.closed,false);assert.equal(result.brep.faces,1);assert.deepEqual(result.brep.edge_ranges,[ranges[0],ranges[1],ranges[0],ranges[1]]);
 if(result.crease)compareIntersection(result.normal,roofNormal(args[0],args[2]>0.5||(args[2]===0.5&&args[5]===0.5)));
 [[args[5],args[7]],[args[6],args[7]],[args[6],args[8]],[args[5],args[8]]].forEach(([u,v],i)=>compareIntersection(result.brep.vertices[i],oracle(args[0],args[1],u,v)));
 for(const section of result.sections){assert.deepEqual(section.parameter_range,ranges[1-section.fixed_axis]);assert.equal(section.parameters[0],section.parameter_range[0]);assert.equal(section.parameters.at(-1),section.parameter_range[1]);const exact=t=>oracle(args[0],args[1],section.fixed_axis===0?section.fixed_parameter:t,section.fixed_axis===1?section.fixed_parameter:t);section.parameters.forEach((t,i)=>compareIntersection(section.samples.slice(i*3,i*3+3),exact(t)));for(let i=0;i<section.error_bounds.length;i++){const bound=section.error_bounds[i];assert.ok(bound>0&&bound<=result.section_chord_error);for(let j=0;j<=8;j++){const t=section.parameters[i]+(section.parameters[i+1]-section.parameters[i])*j/8;assert.ok(pointChordDistance(exact(t),section.samples.slice(i*3,i*3+3),section.samples.slice(i*3+3,i*3+6))<=bound+1e-10);}}}
}
const trimArgs=[35,1,0.5,0.5,0.1,0.2,0.8,0.15,0.85,0];
for(const [index,value] of [[5,-0.1],[6,1.1],[5,0.8],[5,0.9],[7,NaN],[2,0.9],[4,0],[4,NaN],[4,1e-14],[1,0],[9,2]]){const args=trimArgs.slice();args[index]=value;const {status,result}=trimmedSurface(...args);assert.equal(status,1);assert.equal(typeof result.error,'string');}
assert.equal(trimmedSurface(...trimArgs).status,0);
console.log('Rectangular UV restriction: 5 native/WASM retained B-rep cases, original-coordinate geometry and bounded triangles, exact boundaries, smooth/C0 endpoint normals and errors/recovery passed.');
function holedSurface(...args){const status=k.hagane_generate_surface_hole(...args);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
function checkHoledSurface(data){
 const oracle=data.crease?roofPoint:surfacePoint,topology=data.brep,outer=data.outer,hole=data.holes[0];assert.deepEqual(outer,[[0.1,0.9],[0.1,0.9]]);assert.equal(topology.faces,1);assert.equal(topology.closed,false);assert.equal(topology.orientation,1);assert.equal(topology.vertices.length,8);assert.equal(topology.edge_vertices.length,8);assert.equal(topology.wire_edges.length,2);assert.equal(data.sections.length,8);
 topology.wire_edges.forEach((ids,w)=>{assert.equal(ids.length,4);const uv=topology.wire_uvs[w],signed=uv.reduce((sum,p,i)=>{const q=uv[(i+1)%4];return sum+p[0]*q[1]-p[1]*q[0];},0)/2;assert.ok(Math.abs(signed-(w===0?0.64:-(data.hole_width**2)))<1e-12);const directed=ids.map((edge,i)=>topology.wire_forward[w][i]?topology.edge_vertices[edge]:topology.edge_vertices[edge].toReversed());directed.forEach((edge,i)=>{assert.equal(edge[1],directed[(i+1)%4][0]);compareIntersection(topology.vertices[edge[0]],oracle(data.height,data.weight,...uv[i]));});});
 assert.equal(new Set(topology.wire_edges.flat()).size,8);assert.equal(data.positions.length,data.cells*18);assert.equal(data.triangles,data.cells*2);assert.equal(data.triangle_nodes.length,data.triangles);assert.equal(data.triangle_uvs.length,data.triangles);
 const nodes=new Map(),uvNodes=new Map(),edges=new Map();let area=0;
 data.cell_ranges.forEach(([[u0,u1],[v0,v1]],i)=>{
  assert.ok(outer[0][0]<=u0&&u0<u1&&u1<=outer[0][1]&&outer[1][0]<=v0&&v0<v1&&v1<=outer[1][1]);assert.ok(u1<=hole[0][0]||u0>=hole[0][1]||v1<=hole[1][0]||v0>=hole[1][1]);area+=(u1-u0)*(v1-v0);const bound=data.cell_bounds[i];assert.ok(Number.isFinite(bound)&&bound>0&&bound<=data.surface_error);const p=Array.from({length:6},(_,j)=>data.positions.slice(i*18+j*3,i*18+j*3+3));
  for(let a=0;a<=4;a++)for(let b=0;b<=4;b++){const s=a/4,t=b/4,exact=oracle(data.height,data.weight,u0+(u1-u0)*s,v0+(v1-v0)*t),linear=s>=t?p[0].map((x,j)=>(1-s)*x+(s-t)*p[1][j]+t*p[2][j]):p[0].map((x,j)=>(1-t)*x+s*p[2][j]+(t-s)*p[5][j]);assert.ok(Math.hypot(...exact.map((x,j)=>x-linear[j]))<=bound+1e-10);}
 });assert.ok(Math.abs(area-(0.64-data.hole_width**2))<1e-10);
 data.triangle_nodes.forEach((ids,t)=>{const uvs=data.triangle_uvs[t];assert.ok((uvs[1][0]-uvs[0][0])*(uvs[2][1]-uvs[0][1])-(uvs[1][1]-uvs[0][1])*(uvs[2][0]-uvs[0][0])>0);ids.forEach((id,j)=>{const uv=uvs[j],p=data.positions.slice(t*9+j*3,t*9+j*3+3);compareIntersection(p,oracle(data.height,data.weight,...uv));if(nodes.has(id))assert.deepEqual(nodes.get(id),{uv,p});else nodes.set(id,{uv,p});const key=JSON.stringify(uv);if(uvNodes.has(key))assert.equal(uvNodes.get(key),id);else uvNodes.set(key,id);});for(let j=0;j<3;j++){const a=ids[j],b=ids[(j+1)%3],key=[Math.min(a,b),Math.max(a,b)].join(','),edge=edges.get(key)??{a,b,count:0,balance:0};edge.count++;edge.balance+=a<b?1:-1;edges.set(key,edge);}});
 const onRect=(a,b,rect)=>[0,1].some(axis=>rect[axis].some(x=>a[axis]===x&&b[axis]===x)&&[a,b].every(p=>p[1-axis]>=rect[1-axis][0]&&p[1-axis]<=rect[1-axis][1]));let innerEdges=0,outerEdges=0;
 for(const edge of edges.values()){const a=nodes.get(edge.a).uv,b=nodes.get(edge.b).uv,onOuter=onRect(a,b,outer),onHole=onRect(a,b,hole);assert.equal(edge.count,onOuter||onHole?1:2);assert.equal(Math.abs(edge.balance),onOuter||onHole?1:0);if(onOuter)outerEdges++;if(onHole)innerEdges++;}assert.ok(innerEdges>0&&outerEdges>0);assert.equal(nodes.size,data.geometric_vertices);
 for(const section of data.sections){const exact=t=>{const uv=section.pcurve_origin.map((x,i)=>x+t*section.pcurve_direction[i]);return oracle(data.height,data.weight,...uv);};section.parameters.forEach((t,i)=>compareIntersection(section.samples.slice(i*3,i*3+3),exact(t)));for(let i=0;i<section.error_bounds.length;i++){const bound=section.error_bounds[i];assert.ok(bound>0&&bound<=data.section_chord_error);for(let j=0;j<=8;j++){const t=section.parameters[i]+(section.parameters[i+1]-section.parameters[i])*j/8;assert.ok(pointChordDistance(exact(t),section.samples.slice(i*3,i*3+3),section.samples.slice(i*3+3,i*3+6))<=bound+1e-10);}}}
}
for(const args of [[35,1,0.1,0.3,0],[-35,2,0.5,0.4,0],[35,1,0.1,0.3,1],[-35,2,0.5,0.3,1]]){
 const {status,result}=holedSurface(...args);assert.equal(status,0,JSON.stringify(result));const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs_surface_hole','--',...args.map(String)],{encoding:'utf8',maxBuffer:32*1024*1024,cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);checkHoledSurface(result);
}
for(const args of [[35,1,0.1,0,0],[35,1,0.1,-0.1,0],[35,1,0.1,NaN,0],[35,1,0.1,0.8,0],[35,1,0.1,0.9,0],[35,0,0.1,0.3,0],[NaN,1,0.1,0.3,0],[35,1,0,0.3,0],[35,1,NaN,0.3,0],[35,1,1e-14,0.3,0],[35,1,0.1,0.3,2]]){const {status,result}=holedSurface(...args);assert.equal(status,1);assert.equal(typeof result.error,'string');}
assert.equal(holedSurface(35,1,0.1,0.3,0).status,0);
console.log('Rectangular UV holes: 4 native/WASM cases, exact inner-wire winding/geometry, excluded UV area, independent triangle bounds, welded edge incidence and errors/recovery passed.');

// Independent BigInt oracle: decode exact IEEE-754 coordinates to integers
// in units of 2^-1074. No floating arithmetic in the reference determinant.
const bitsView=new DataView(new ArrayBuffer(8));
function dyadic(value){bitsView.setFloat64(0,value);const bits=bitsView.getBigUint64(0);const exponent=Number((bits>>52n)&2047n);const mantissa=(bits&((1n<<52n)-1n))|(exponent?1n<<52n:0n);const integer=mantissa<<BigInt(Math.max(0,exponent-1));return bits>>63n?-integer:integer;}
function exactOrientation(a,b,c){const [ax,ay,bx,by,cx,cy]=[...a,...b,...c].map(dyadic);const det=(bx-ax)*(cy-ay)-(by-ay)*(cx-ax);return det>0n?1:det<0n?-1:0;}
let predicateCases=0;
function checkOrientation(a,b,c){const expected=exactOrientation(a,b,c);assert.equal(k.hagane_orient2d(...a,...b,...c),expected);assert.equal(k.hagane_orient2d(...c,...b,...a),-expected||0);predicateCases++;}
for(const s of [Number.MIN_VALUE,2**-1022,1e-200,1,1e200,Number.MAX_VALUE]){checkOrientation([0,-0],[s,0],[0,s]);checkOrientation([-s,-s],[s,-s],[s,s]);checkOrientation([s,0],[s,0],[0,s]);}
for(let exponent=-1000;exponent<=900;exponent+=5){const s=2**exponent,n=134217728;checkOrientation([0,0],[(n+1)*s,n*s],[n*s,(n-1)*s]);}
let randomBits=0xcafebabe12345678n;
function random64(){randomBits=BigInt.asUintN(64,randomBits*6364136223846793005n+1442695040888963407n);return randomBits;}
function finiteFloat(){let bits=random64();if(((bits>>52n)&2047n)===2047n)bits^=1n<<52n;bitsView.setBigUint64(0,bits);return bitsView.getFloat64(0);}
for(let i=0;i<2000;i++){checkOrientation([finiteFloat(),finiteFloat()],[finiteFloat(),finiteFloat()],[finiteFloat(),finiteFloat()]);}
for(let i=0;i<500;i++){
 const a=[finiteFloat(),finiteFloat()],b=[finiteFloat(),finiteFloat()];
 checkOrientation(a,b,a);checkOrientation(a,b,b);
}
assert.equal(k.hagane_orient2d(NaN,0,1,0,0,1),2);assert.equal(k.hagane_orient2d(0,0,Infinity,0,0,1),2);
function onBox(p,a,b){return [0,1].every(i=>p[i]>=Math.min(a[i],b[i])&&p[i]<=Math.max(a[i],b[i]));}
function exactSegments(a,b,c,d){const [s1,s2,s3,s4]=[exactOrientation(a,b,c),exactOrientation(a,b,d),exactOrientation(c,d,a),exactOrientation(c,d,b)];return s1*s2<0&&s3*s4<0||s1===0&&onBox(c,a,b)||s2===0&&onBox(d,a,b)||s3===0&&onBox(a,c,d)||s4===0&&onBox(b,c,d);}
for(let i=0;i<200;i++){const points=Array.from({length:4},()=>[finiteFloat(),finiteFloat()]);assert.equal(k.hagane_segments_intersect2d(...points.flat()),Number(exactSegments(...points)));}
for(const points of [[[0,0],[1,0],[1,0],[2,0]],[[0,0],[3,0],[1,0],[2,0]],[[0,0],[1,0],[1+Number.EPSILON,0],[2,0]],[[1,1],[1,1],[0,0],[2,2]]]){assert.equal(k.hagane_segments_intersect2d(...points.flat()),Number(exactSegments(...points)));}
assert.equal(k.hagane_segments_intersect2d(0,0,1,0,NaN,0,2,0),2);
function numericalDemo(scale,angle){const status=k.hagane_generate_predicates(scale,angle);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const [scale,angle,classification] of [[10,0.1,'point'],[10,1e-12,'parallel'],[1e-12,1e-12,'coincident'],[1e8,0.1,'point'],[1,Math.PI/2,'point']]){
 const {status,result}=numericalDemo(scale,angle);assert.equal(status,0);assert.equal(result.orientation,-1);assert.equal(result.line_plane,classification);assert.equal(result.length_budget,Math.max(1e-8,1e-10*scale));
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','predicates','--',String(scale),String(angle)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
 for(const field of ['orientation','length_budget','line_plane'])assert.equal(native[field],result[field]);
 if(result.point){result.point.forEach((value,i)=>assert.ok(Math.abs(value-native.point[i])<=1e-10*Math.max(1,Math.abs(value))));assert.ok(Math.abs(result.parameter-native.parameter)<=1e-10*Math.max(1,Math.abs(result.parameter)));}
}
for(const [scale,angle] of [[0,0.1],[-1,0.1],[NaN,0.1],[1,NaN]]){assert.equal(numericalDemo(scale,angle).status,1);}
assert.equal(numericalDemo(10,0.1).status,0);
console.log(`Predicates: ${predicateCases} exact BigInt orientation cases, 204 exact segment cases, tolerance/classification native/WASM parity and errors passed.`);
function intersections(mode,offset,placement=0){const status=k.hagane_generate_intersections(mode,offset,placement);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
function compareIntersection(a,b){if(typeof a==='number'){assert.ok(Number.isFinite(a));assert.ok(Math.abs(a-b)<=1e-11*Math.max(1,Math.abs(a),Math.abs(b)));}else if(a && typeof a==='object'){assert.deepEqual(Object.keys(a),Object.keys(b));for(const key of Object.keys(a))compareIntersection(a[key],b[key]);}else assert.equal(a,b);}
let intersectionCases=0;
for(const mode of [0,1,2])for(const offset of (mode===0?[-3,-1.5,0,1,2,3]:mode===1?[0,5e-9,1]:[0,1,2,3]))for(const placement of (mode===2||Math.abs(offset)===2?[0]:[0,0.7,1.5])){
 const {status,result}=intersections(mode,offset,placement);assert.equal(status,0,JSON.stringify({mode,offset,placement,result}));
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','intersections','--',String(mode),String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 assert.equal(result.planes.kind,mode===1?(offset<=1e-8?'coincident':'parallel'):'line');
 if(result.planes.kind==='line'){assert.ok(Math.abs(Math.hypot(...result.planes.direction)-1)<1e-12);}
 if(mode===2){assert.equal(result.cylinder.kind,offset===2?'coincident':'empty');if(offset===2){assert.deepEqual(result.cylinder.range,[-0.5,1.5]);assert.equal(result.cylinder.angle,0);}}
 else if(Math.abs(offset)>2)assert.equal(result.cylinder.kind,'empty');
 else{const hits=result.cylinder.points;assert.equal(hits.length,Math.abs(offset)===2?1:2);const half=Math.sqrt(4-offset**2);hits.forEach((h,i)=>{assert.ok(Math.abs(h.parameter-(5+(i===0?-half:half))/2)<1e-11);assert.equal(h.contact,half===0?'tangent':'crossing');assert.ok(Math.abs(h.uv[1]-2)<1e-11);assert.ok(h.uv[0]>=0&&h.uv[0]<2*Math.PI);});}
 intersectionCases++;
}
for(const args of [[0,2+1e-9,0],[0,2-1e-9,0],[2,2+1e-9,0],[3,1,0],[0,NaN,0],[0,1,Infinity]]){const {status,result}=intersections(...args);assert.equal(status,1);assert.equal(typeof result.error,'string');}
assert.equal(intersections(0,1,0.7).status,0);
console.log(`Intersections: ${intersectionCases} native/WASM plane/line-cylinder cases, independent roots, UV, classifications, errors and recovery passed.`);
function faceClipping(offset,placement=0){const status=k.hagane_generate_face_clipping(offset,placement);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
let clippingCases=0;
for(const offset of [0,0.5,0.9,1.5,2.5,3.5,4.5,-0.5,-2.5])for(const placement of [0,0.7]){
 const {status,result}=faceClipping(offset,placement);assert.equal(status,0,JSON.stringify({offset,placement,result}));
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','face_clipping','--',String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 const insideHole=Math.abs(offset)<1,insideCap=Math.abs(offset)<3;
 assert.equal(result.intervals.length,insideHole?2:insideCap?1:0);assert.equal(result.events.length,insideHole?4:insideCap?2:0);
 const expected=insideHole?[[3,(10-Math.sqrt(1-offset**2))/2],[(10+Math.sqrt(1-offset**2))/2,7]]:insideCap?[[3,7]]:[];compareIntersection(result.intervals,expected);
 result.events.forEach((e,i)=>{assert.ok(e.edge_parameter>=0);assert.ok(Number.isInteger(e.edge)&&Number.isInteger(e.wire)&&Number.isInteger(e.coedge));if(i)assert.ok(e.parameter>result.events[i-1].parameter);});
 assert.equal(result.segments.length,Math.abs(offset)<1?2:Math.abs(offset)<4?1:0);
 const length=result.segments.reduce((sum,s)=>sum+Math.hypot(...s.start.map((v,i)=>s.end[i]-v)),0);const expectedLength=Math.abs(offset)<1?4-2*Math.sqrt(1-offset**2):Math.abs(offset)<4?4:0;assert.ok(Math.abs(length-expectedLength)<1e-10);
 for(const segment of result.segments){assert.ok(segment.range[1]>segment.range[0]);for(const pc of [segment.first_uv,segment.second_uv])assert.ok(Math.abs(Math.hypot(...pc.direction)-(segment.range[1]-segment.range[0]))<1e-10);}
 assert.ok(Math.abs(result.volume-(96-2*Math.PI))<1e-10);clippingCases++;
}
for(const [offset,placement] of [[1,0],[1+1e-9,0],[1-1e-9,0],[3,0],[4,0],[NaN,0],[0,Infinity]]){const {status,result}=faceClipping(offset,placement);assert.equal(status,1);assert.equal(typeof result.error,'string');}
assert.equal(faceClipping(0.5,0.7).status,0);
console.log(`Face clipping: ${clippingCases} native/WASM analytic intervals, boundary provenance, finite pcurves, volume preservation, unsupported contacts and recovery passed.`);
for(const offset of [8,14,24,-14]){const {status,result}=preset(7,offset);assert.equal(status,0);assert.equal(result.faces,8);assert.equal(result.edges,18);assert.ok(Math.abs(result.volume-(115200-Math.PI*49*24))<1e-8);}
for(const offset of [0,7,7+1e-9,30,31,NaN,Infinity])assert.equal(preset(7,offset).status,1);
assert.equal(preset(7,14,0).status,1);assert.equal(preset(7).status,0);
assert.notDeepEqual(preset(7,8).result.positions,preset(7,24).result.positions);
console.log('Face splitting: B-rep subdivision, unchanged volume, cut offsets, errors and recovery passed.');
for(const offset of [8,14,24,-14]){const {status,result}=preset(8,offset);assert.equal(status,0);assert.equal(result.faces,13);assert.equal(result.edges,31);assert.ok(Math.abs(result.volume-(4800-(4-Math.PI)*576)*24)<1e-8);}
for(const offset of [6,30,31,NaN,Infinity])assert.equal(preset(8,offset).status,1);
assert.equal(preset(8,14,0).status,1);assert.equal(preset(8).status,0);
assert.notDeepEqual(preset(8,8).result.positions,preset(8,24).result.positions);
console.log('Arc face splitting: shared rims, rectangular cylinder walls, unchanged volume, offsets, errors and recovery passed.');

for(const slider of [8,14,24]){const {status,result}=preset(9,slider);assert.equal(status,0);assert.equal(result.faces,15);assert.equal(result.edges,45);assert.equal(result.volume,103680);}
for(const slider of [6,26,NaN,Infinity])assert.equal(preset(9,slider).status,1);
assert.equal(preset(9,14,0).status,1);assert.equal(preset(9).status,0);
assert.notDeepEqual(preset(9,8).result.positions,preset(9,24).result.positions);
console.log('Hole-cut graph: native/WASM parity, offsets, rejection and recovery passed.');

for(const offset of [8,14,24,-14]){const {status,result}=preset(10,offset);assert.equal(status,0);assert.equal(result.faces,11);assert.equal(result.edges,26);assert.ok(Math.abs(result.volume-5376*Math.PI)<1e-8);}
for(const offset of [0,26,26-1e-9,30,NaN,Infinity])assert.equal(preset(10,offset).status,1);
assert.equal(preset(10,14,0).status,1);assert.equal(preset(10).status,0);
assert.notDeepEqual(preset(10,8).result.positions,preset(10,24).result.positions);
console.log('Repeated arc crossings: annular topology, offsets, native/WASM parity, errors and recovery passed.');

for(const slider of [8,14,16,24]){const {status,result}=preset(11,slider);assert.equal(status,0);assert.equal(result.faces,13);assert.equal(result.edges,34);assert.ok(Math.abs(result.volume-(115200-3456*Math.PI))<1e-8);}
for(const slider of [4,4+1e-9,28,46,47,NaN,Infinity])assert.equal(preset(11,slider).status,1);
assert.equal(preset(11,14,0).status,1);assert.equal(preset(11).status,0);
assert.notDeepEqual(preset(11,8).result.positions,preset(11,24).result.positions);
console.log('Periodic rim splitting: seam passage, bore topology, offsets, native/WASM parity, errors and recovery passed.');

for(const value of [8,14,24]){const {status,result}=preset(12,value);assert.equal(status,0);assert.equal(result.volume,115200);assert.equal(result.faces,6);assert.equal(result.edges,13);}
for(const value of [0,7,25,NaN,Infinity])assert.equal(preset(12,value).status,1);
assert.equal(preset(12,14,0).status,1);assert.equal(preset(12).status,0);
assert.notDeepEqual(preset(12,8).result.positions,preset(12,24).result.positions);
console.log('Planar sewing: shared subdivision, metrics, native/WASM parity, errors and recovery passed.');
function classifySolid(x,y,z){const status=k.hagane_classify_demo(x,y,z);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const [x,y,z,expected] of [[-10,0,0,'inside'],[-26,0,0,'outside'],[10,10,0,'outside'],[-10,0,12,'boundary'],[-40,-30,-12,'boundary'],[45,0,0,'outside'],[-20,0,0,'boundary'],[-26,0,12,'outside'],[-10,0,12+1e-9,'boundary'],[-10,0,12+1e-6,'outside']]){
 const {status,result}=classifySolid(x,y,z);assert.equal(status,0);assert.equal(result.location,expected);assert.equal(result.volume,69336);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','classification','--',...[x,y,z].map(String)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);
}
for(const p of [[NaN,0,0],[0,Infinity,0],[0,0,-Infinity]])assert.equal(classifySolid(...p).status,1);
assert.equal(classifySolid(-10,0,0).status,0);
assert.equal(k.hagane_classification_mesh(),0);
const classMesh=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));
const nativeClassMesh=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','classification','--','--mesh'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(classMesh,nativeClassMesh);
console.log('Solid classification: analytic probes, native/WASM parity, boundary bands, display fixture, invalid points and recovery passed.');

function partition(offset){const status=k.hagane_partition_demo(offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const offset of [-8,-2,0,8]){const {status,result}=partition(offset);assert.equal(status,0);assert.equal(result.negative.volume,4800*(offset+12));assert.equal(result.positive.volume,4800*(12-offset));assert.equal(result.negative.volume+result.positive.volume,result.original_volume);assert.equal(result.section_faces,1);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','solid_split','--',String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);}
for(const offset of [-12,-12+1e-9,12,13,NaN,Infinity])assert.equal(partition(offset).status,1);
assert.equal(partition(0).status,0);assert.equal(preset(13,24).result.volume,19200);
console.log('Solid plane partition: two closed parts, analytic volume conservation, native/WASM parity, contacts and recovery passed.');

function convexVolume(offset){const d=32*Math.SQRT2;return (4096-Math.max(d-40+offset,0)**2-Math.max(d-40-offset,0)**2-2*(d-30)**2)*24;}
function commonSolid(offset){const status=k.hagane_convex_intersection_demo(offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const offset of [-8,-2,0,8]){const {status,result}=commonSolid(offset);assert.equal(status,0);assert.equal(result.kind,'solid');assert.ok(Math.abs(result.mesh.volume-convexVolume(offset))<1e-8);assert.equal(result.mesh.faces,Math.abs(offset)>6?9:10);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','convex_intersection','--',String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);}
assert.deepEqual(commonSolid(100),{status:0,result:{kind:'empty'}});
for(const offset of [32*Math.SQRT2-40,70-32*Math.SQRT2,NaN,Infinity])assert.equal(commonSolid(offset).status,1);
assert.equal(commonSolid(0).status,0);
console.log('Convex solid intersection: independent clipped-diamond volume, native/WASM parity, empty/contact results and recovery passed.');

function subtractSolid(offset){const status=k.hagane_convex_difference_demo(offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const offset of [-8,-2,0,8]){const {status,result}=subtractSolid(offset);assert.equal(status,0);assert.equal(result.kind,'solid');assert.equal(result.mesh.volume,107520);assert.equal(result.mesh.faces,20);assert.equal(result.mesh.edges,44);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','convex_difference','--',String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);}
assert.equal(subtractSolid(100).result.mesh.volume,115200);
for(const offset of [30,50,50-1e-9,NaN,Infinity])assert.equal(subtractSolid(offset).status,1);
assert.equal(subtractSolid(0).status,0);assert.equal(preset(15,14,0).status,1);assert.equal(preset(15).status,0);
console.log('Convex operand difference: closed through-hole, analytic volume, native/WASM parity, unchanged/contact results and recovery passed.');
function uniteSolid(offset){const status=k.hagane_convex_union_demo(offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const offset of [-8,-2,0,8]){
 const {status,result}=uniteSolid(offset);assert.equal(status,0);assert.ok(Math.abs(result.volume-(176000+640*offset))<1e-8);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','convex_union','--',String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);
}
for(const offset of [30,30-1e-9,100,NaN,Infinity]){const {status,result}=uniteSolid(offset);assert.equal(status,1);assert.ok(result.error);}
assert.equal(uniteSolid(-2).status,0);
console.log('Convex union: analytic combined volume, native/WASM parity, disjoint/contact errors and recovery passed.');
function contactBoxes(offset){const status=k.hagane_box_contact_demo(offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const offset of [-8,-2,0,8]){
 const {status,result}=contactBoxes(offset);assert.equal(status,0);assert.ok(Math.abs(result.volume-122880)<1e-8);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','box_contact','--',String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);
}
for(const offset of [40,40-1e-9,100,NaN,Infinity]){const {status,result}=contactBoxes(offset);assert.equal(status,1);assert.ok(result.error);}
assert.equal(contactBoxes(-2).status,0);
console.log('Box face contact: analytic volume, native/WASM parity, edge contact/disconnected/near-contact errors and recovery passed.');
function mergedContact(offset){const status=k.hagane_merged_contact_demo(offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const offset of [-8,-2,0,8]){
 const {status,result}=mergedContact(offset);assert.equal(status,0);assert.ok(Math.abs(result.volume-122880)<1e-8);assert.equal(result.faces,10);assert.equal(result.edges,34);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','face_merge','--',String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);
}
for(const offset of [40,40-1e-9,100,NaN,Infinity]){const {status,result}=mergedContact(offset);assert.equal(status,1);assert.ok(result.error);}
assert.equal(mergedContact(-2).status,0);
console.log('Coplanar face merging: reduced topology, unchanged analytic volume, native/WASM parity, errors and recovery passed.');
function exactOrientation3d(a,b,c,d){const aa=a.map(dyadic);const diff=p=>p.map((v,i)=>dyadic(v)-aa[i]);const u=diff(b),v=diff(c),w=diff(d);const det=(u[1]*v[2]-u[2]*v[1])*w[0]+(u[2]*v[0]-u[0]*v[2])*w[1]+(u[0]*v[1]-u[1]*v[0])*w[2];return det>0n?1:det<0n?-1:0;}
let spatialCases=0;
function checkOrientation3d(a,b,c,d){const expected=exactOrientation3d(a,b,c,d);assert.equal(k.hagane_orient3d(...a,...b,...c,...d),expected);assert.equal(k.hagane_orient3d(...a,...c,...b,...d),-expected||0);spatialCases++;}
for(const s of [Number.MIN_VALUE,2**-1022,1e-200,1,1e200,Number.MAX_VALUE]){checkOrientation3d([0,0,0],[s,0,0],[0,s,0],[0,0,s]);checkOrientation3d([-s,-s,-s],[s,-s,-s],[-s,s,-s],[-s,-s,s]);}
for(let exponent=-1000;exponent<=900;exponent+=10){const s=2**exponent,n=134217728;checkOrientation3d([0,0,0],[(n+1)*s,n*s,0],[n*s,(n-1)*s,0],[0,0,s]);}
for(let i=0;i<1000;i++){const p=Array.from({length:4},()=>Array.from({length:3},finiteFloat));checkOrientation3d(...p);if(i%2===0)checkOrientation3d(p[0],p[1],p[2],p[0]);}
assert.equal(k.hagane_orient3d(NaN,0,0,0,0,0,0,0,0,0,0,0),2);assert.equal(k.hagane_orient3d(0,0,0,0,0,0,Infinity,0,0,0,0,0),2);
console.log(`Exact 3D orientation: ${spatialCases} independent BigInt cases, permutations, full exponent range and invalid inputs passed.`);
function reframedMerge(offset){const status=k.hagane_reframed_merge_demo(offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const offset of [-8,-2,0,8]){
 const {status,result}=reframedMerge(offset);assert.equal(status,0);assert.ok(Math.abs(result.volume-(176000+640*offset))<1e-8);assert.equal(result.faces,12);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','reframed_merge','--',String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
}
for(const offset of [30,30-1e-9,100,NaN,Infinity]){const {status,result}=reframedMerge(offset);assert.equal(status,1);assert.ok(result.error);}
assert.equal(reframedMerge(-2).status,0);
console.log('Independent tilted UV frames: exact plane identity, merged topology, analytic volume, native/WASM parity and error recovery passed.');
function simplifiedContact(offset){const status=k.hagane_simplified_contact_demo(offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const offset of [-8,-2,0,8]){
 const {status,result}=simplifiedContact(offset);assert.equal(status,0);assert.ok(Math.abs(result.volume-122880)<1e-8);assert.equal(result.faces,10);assert.equal(result.edges,24);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','edge_simplify','--',String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);
}
for(const offset of [40,40-1e-9,100,NaN,Infinity]){const {status,result}=simplifiedContact(offset);assert.equal(status,1);assert.ok(result.error);}
assert.equal(simplifiedContact(-2).status,0);
console.log('Shared straight edge simplification: reduced topology, unchanged analytic volume, native/WASM parity, errors and recovery passed.');
function curvedClassify(model,p){const status=k.hagane_classify_curved_demo(model,...p);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const [model,probes] of [[0,[[[-20,0,0],'inside'],[[0,0,0],'outside'],[[14,0,0],'boundary'],[[0,0,12],'outside'],[[-20,0,12],'boundary'],[[45,0,0],'outside']]], [1,[[[12,0,0],'inside'],[[0,0,0],'inside'],[[24,0,0],'boundary'],[[24,0,12],'boundary'],[[35,0,0],'outside']]], [2,[[[20,0,0],'inside'],[[0,0,0],'outside'],[[14,0,0],'boundary'],[[20,0,12],'boundary'],[[35,0,0],'outside']]], [3,[[[0,0,0],'inside'],[[39,29,0],'outside'],[[40,16,0],'boundary'],[[0,0,12],'boundary']]], [4,[[[-20,0,0],'inside'],[[0,-10,0],'outside'],[[0,16,0],'boundary'],[[-20,0,12],'boundary']]], [5,[[[26,-3,0],'inside'],[[6,-3,0],'outside'],[[34,19,0],'boundary'],[[32,-6,12],'boundary'],[[44,8,12],'boundary'],[[45,30,0],'outside']]]]){
 for(const [p,expected] of probes){const {status,result}=curvedClassify(model,p);assert.equal(status,0);assert.equal(result.location,expected);const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','curved_classification','--',String(model),...p.map(String)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);}
 assert.equal(k.hagane_curved_classification_mesh(model),0);const mesh=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','curved_classification','--',String(model),'--mesh'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(mesh,native);
}
for(const [p,expected] of [[[26,-3,0],'inside'],[[6,-3,0],'outside'],[[34,19,0],'boundary'],[[32,-6,12],'boundary'],[[44,8,12],'boundary'],[[45,30,0],'outside']]){
 const {status,result}=curvedClassify(6,p);assert.equal(status,0);assert.equal(result.location,expected);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','curved_classification','--','6',...p.map(String)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);
}
assert.equal(k.hagane_curved_classification_mesh(6),0);
const harmonicMesh=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));
const harmonicNativeMesh=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','curved_classification','--','6','--mesh'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(harmonicMesh,harmonicNativeMesh);
assert.equal(harmonicMesh.faces,34);assert.equal(harmonicMesh.edges,80);
assert.equal(curvedClassify(6,[NaN,0,0]).status,1);assert.equal(curvedClassify(6,[26,-3,0]).status,0);
console.log('Harmonic solid classification: material, hole, wall, cap, vertex, native/WASM mesh parity, errors and recovery passed.');
for(const [offset,expected] of [[-1.4e-8,'inside'],[-0.6e-8,'boundary'],[0.6e-8,'boundary'],[1.4e-8,'outside']]){const p=[34+0.6*offset/Math.sqrt(1.01),19+0.8*offset/Math.sqrt(1.01),-0.1*offset/Math.sqrt(1.01)];const {status,result}=curvedClassify(5,p);assert.equal(status,0);assert.equal(result.location,expected);const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','curved_classification','--','5',...p.map(String)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.deepEqual(result,native);}
for(const [offset,expected] of [[-1.4e-8,'inside'],[-0.6e-8,'boundary'],[0.6e-8,'boundary'],[1.4e-8,'outside']]){
 const p=[34+0.6*offset/Math.sqrt(1.01),19+0.8*offset/Math.sqrt(1.01),-0.1*offset/Math.sqrt(1.01)];
 const {status,result}=curvedClassify(6,p);assert.equal(status,0);assert.equal(result.location,expected);
}
for(const p of [[NaN,0,0],[0,Infinity,0]])assert.equal(curvedClassify(5,p).status,1);assert.equal(curvedClassify(5,[26,-3,0]).status,0);
assert.equal(curvedClassify(999,[0,0,0]).status,1);assert.equal(curvedClassify(0,[NaN,0,0]).status,1);assert.equal(curvedClassify(1,[0,Infinity,0]).status,1);assert.equal(curvedClassify(0,[-20,0,0]).status,0);
console.log('Curved solid classification: normal/skew walls, rounded holes, material, voids, caps, rims, native/WASM parity, invalid inputs and recovery passed.');

for(const radius of [8,14,24]){const {status,result}=preset(21,radius);assert.equal(status,0);assert.ok(Math.abs(result.volume-(4600+(4-Math.PI)*9-Math.PI*radius**2/2)*24)<1e-8);const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','framed_arc_extrusion','--',String(radius)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);}
for(const radius of [0,-1,30,NaN,Infinity])assert.equal(preset(21,radius).status,1);
assert.equal(preset(21,14,0).status,1);assert.equal(preset(21).status,0);
console.log('Framed mixed extrusion: tilted negative normal, analytic volume, native/WASM meshes, invalid input and recovery passed.');

function skewArc(radius,offset,height){const status=k.hagane_skew_arc_extrusion_demo(radius,offset,height);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const [radius,offset,height] of [[8,8,-24],[14,14,-24],[24,24,-24],[14,-14,24],[14,1e-9,24],[14,0,-24]]){
 const {status,result}=skewArc(radius,offset,height);assert.equal(status,0);assert.ok(Math.abs(result.volume-(4600+(4-Math.PI)*9-Math.PI*radius**2/2)*Math.abs(height))<1e-8);assert.equal(result.faces,16);assert.equal(result.edges,42);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','skew_arc_extrusion','--',String(radius),String(offset),String(height)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
}
for(const args of [[0,14,-24],[30,14,-24],[NaN,14,-24],[14,NaN,-24],[14,Infinity,-24],[14,14,0],[14,14,1e-9],[14,14,Infinity],[14,14,NaN]]){const {status,result}=skewArc(...args);assert.equal(status,1);assert.ok(result.error);}
assert.equal(skewArc(14,14,-24).status,0);
console.log('Skew arc extrusion: exact walls, signed spans, analytic volume, native/WASM geometry, errors and recovery passed.');
function extrusionHits(mode,offset,placement){const status=k.hagane_extrusion_intersections_demo(mode,offset,placement);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const [mode,offset,placement] of [[0,-30,0],[0,-14,0],[0,0,0],[0,14,0],[0,24,0],[0,30,0],[0,14,0.7],[0,-14,-0.5],[1,0,0],[2,0,0],[1,1,0],[2,-1,0]]){
 const {status,result}=extrusionHits(mode,offset,placement);assert.equal(status,0);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','extrusion_intersections','--',String(mode),String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 assert.ok(Math.abs(result.mesh.volume-24*24*Math.PI*24)<1e-8);assert.equal(result.mesh.faces,4);assert.equal(result.mesh.edges,6);
 const hit=result.intersection;
 if(mode===0){if(Math.abs(offset)>24){assert.equal(hit.kind,'empty');}else {assert.equal(hit.kind,'points');const half=Math.sqrt(24**2-offset**2);const expected=half===0?[20]:[(40-half)/2,(40+half)/2];assert.equal(hit.hits.length,expected.length);hit.hits.forEach((h,i)=>{assert.ok(Math.abs(h.parameter-expected[i])<1e-10);assert.ok(Math.abs(h.uv[1]-12)<1e-10);assert.equal(h.contact,half===0?'tangent':'crossing');});}}
 else if(offset===0){assert.equal(hit.kind,'coincident');assert.ok(Math.abs(hit.range[0])<1e-12 && Math.abs(hit.range[1]-1)<1e-12);}else assert.equal(hit.kind,'empty');
}
for(const args of [[3,0,0],[0,NaN,0],[0,Infinity,0],[0,0,NaN],[0,24-1e-9,0],[0,24+1e-9,0],[1,1e-9,0],[2,-1e-9,0]]){const {status,result}=extrusionHits(...args);assert.equal(status,1);assert.ok(result.error);}
assert.equal(extrusionHits(0,14,0).status,0);
console.log('Skew circular surface intersections: independent roots, contacts, signed generator intervals, placement, native/WASM mesh/query parity, errors and recovery passed.');

function faceHits(selection,mode,offset,placement){const status=k.hagane_circular_face_intersections_demo(selection,mode,offset,placement);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const [selection,mode,offset,placement] of [[0,0,14,0],[0,0,-14,0],[1,0,-14,0],[1,0,14,0],[0,0,24,0],[0,0,-24,0],[1,0,-24,0],[0,0,0,0],[0,1,0,0],[0,2,0,0],[1,1,0,0],[1,2,0,0],[0,0,14,0.7],[1,0,-14,-0.5]]){
 const {status,result}=faceHits(selection,mode,offset,placement);assert.equal(status,0);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','circular_face_intersections','--',String(selection),String(mode),String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 assert.equal(result.face,2+selection);assert.ok(result.display_mesh.positions.length>0 && result.display_mesh.positions.length<result.mesh.positions.length);assert.equal(result.display_mesh.normals.length,result.display_mesh.positions.length);
 const hit=result.intersection;
 if(mode===0){if((selection===0 && offset<0)||(selection===1 && offset>0)){assert.equal(hit.kind,'empty');}else {assert.equal(hit.kind,'points');const half=Math.sqrt(24**2-offset**2);const expected=half===0?[20]:[(40-half)/2,(40+half)/2];assert.equal(hit.hits.length,expected.length);hit.hits.forEach((p,i)=>assert.ok(Math.abs(p.parameter-expected[i])<1e-10));if(offset===0){assert.ok(hit.hits.every(p=>p.boundaries.length===1 && p.boundaries[0].edge_parameter===0.5));}else assert.ok(hit.hits.every(p=>p.boundaries.length===0));}}
 else {assert.equal(hit.kind,'coincident');assert.ok(Math.abs(hit.range[0])<1e-12 && Math.abs(hit.range[1]-1)<1e-12);assert.ok(hit.endpoints.every(p=>p.boundaries.length===2));}
}
for(const args of [[2,0,14,0],[0,3,14,0],[0,0,NaN,0],[0,0,14,Infinity],[0,0,1e-9,0],[0,0,-1e-9,0],[0,1,1e-9,0],[1,1,-1e-9,0]]){const {status,result}=faceHits(...args);assert.equal(status,1);assert.ok(result.error);}
assert.equal(faceHits(0,0,14,0).status,0);
console.log('Circular face intersections: angular trimming, original edge provenance, certified shared-edge generators, native/WASM query/mesh parity, invalid inputs and recovery passed.');

function skewSubdivision(fraction,placement=0){const status=k.hagane_skew_face_subdivision_demo(fraction,placement);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const [fraction,placement] of [[0.25,0],[14/32,0],[0.75,0],[0.4,0.7],[0.6,-0.5]]){
 const {status,result}=skewSubdivision(fraction,placement);assert.equal(status,0);assert.equal(result.faces,22);assert.equal(result.edges,58);assert.ok(Math.abs(result.volume-(64*48-(4-Math.PI)*100-(20*16-(4-Math.PI)*16))*24)<1e-8);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','skew_face_subdivision','--',String(fraction),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
}
for(const fraction of [0,1,-0.1,1e-12,NaN,Infinity])assert.equal(skewSubdivision(fraction).status,1);
assert.equal(skewSubdivision(0.4,NaN).status,1);assert.equal(skewSubdivision(0.4,Infinity).status,1);assert.equal(skewSubdivision(0.4).status,0);
assert.equal(preset(23,14,0).status,1);assert.equal(preset(23,14).status,0);
console.log('Skew circular face subdivision: cap/wall refinement, closed geometry, analytic volume, native/WASM parity, conditioning errors and recovery passed.');

function obliqueBoundary(slope,placement=0){const status=k.hagane_oblique_boundary_demo(slope,placement);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const [slope,placement] of [[-1/6,0],[-1/24,0],[0,0],[1/6,0],[0.13,0.7],[-0.1,-0.5]]){
 const {status,result}=obliqueBoundary(slope,placement);assert.equal(status,0);assert.equal(result.faces,34);assert.equal(result.edges,80);assert.ok(Math.abs(result.volume-(64*48-(4-Math.PI)*100-(20*16-(4-Math.PI)*16))*24)<1e-8);assert.ok(result.section_segments.length>16);assert.ok(result.section_segments.flat(2).every(Number.isFinite));
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','oblique_boundary','--',String(slope),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 if(placement===0)for(const segment of result.section_segments)for(const p of segment)assert.ok(Math.abs(slope*(p[0]-6)-0.08*(p[1]+3)+p[2])<1e-9);
}
for(const slope of [3,-3,NaN,Infinity])assert.equal(obliqueBoundary(slope).status,1);
assert.equal(obliqueBoundary(0.1,NaN).status,1);assert.equal(obliqueBoundary(0.1,Infinity).status,1);assert.equal(obliqueBoundary(0.1).status,0);
assert.equal(preset(24,14,0).status,1);assert.equal(preset(24,14).status,0);
console.log('Oblique plane boundary subdivision: exact ellipse sections, bounded shared geometry, volume, native/WASM mesh/contour parity, invalid cuts and recovery passed.');
function harmonicFace(selection,mode,offset,placement=0){
 const status=k.hagane_harmonic_face_intersections_demo(selection,mode,offset,placement);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));
 return {status,result};
}
for(const placement of [0,0.37])for(const selection of [0,1])for(const [mode,offset] of (placement===0?[[0,14],[1,0],[2,0],[3,0]]:[[0,14],[3,0]])){
 const {status,result}=harmonicFace(selection,mode,offset,placement);assert.equal(status,0);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','harmonic_face_intersections','--',String(selection),String(mode),String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
 assert.equal(result.intersection.kind,native.intersection.kind);
 for(const field of ['positions','normals']){assert.equal(result.display_mesh[field].length,native.display_mesh[field].length);native.display_mesh[field].forEach((v,i)=>assert.ok(Math.abs(v-result.display_mesh[field][i])<1e-9));}
 const points=result.intersection.endpoints??result.intersection.hits;
 const reference=native.intersection.endpoints??native.intersection.hits;
 assert.equal(points.length,reference.length);points.forEach((p,i)=>assert.ok(Math.abs(p.parameter-reference[i].parameter)<1e-10));
 if(mode===0){assert.equal(points.length,1);assert.ok(Math.abs(points[0].parameter-(40+(selection===0?-1:1)*Math.sqrt(576-196))/2)<1e-10);}
 if(mode===3){assert.equal(points.length,1);assert.equal(points[0].boundaries.length,1);assert.ok(Math.abs(points[0].boundaries[0].edge_parameter-Math.PI/2)<1e-10);}
}
for(const args of [[0,1,0,0.37],[1,2,0,0.37],[2,0,14,0],[0,4,0,0],[0,0,NaN,0],[0,0,14,Infinity],[0,3,1e-9,0],[1,3,-1e-9,0]])assert.equal(harmonicFace(...args).status,1);
assert.equal(harmonicFace(0,3,0).status,0);
console.log('Harmonic face queries: native/WASM roots, ellipse provenance, placement, errors and recovery passed.');
function ellipsePlanar(offset,placement=0){
 const status=k.hagane_ellipse_planar_demo(offset,placement);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};
}
for(const placement of [0,0.37])for(const offset of [-14,14,30]){
 const {status,result}=ellipsePlanar(offset,placement);assert.equal(status,0);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','ellipse_planar','--',String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
 const hits=result.intersection.hits;assert.equal(hits.length,offset===30?0:2);
 const half=24*Math.sqrt(1.0625)*Math.sqrt(1-(offset/24)**2);
 if(offset!==30){assert.equal(result.intersection.intervals.length,1);hits.forEach((p,i)=>{
  assert.ok(Math.abs(p.parameter-(34+(i===0?-half:half))/2)<1e-10);assert.equal(p.boundaries.length,1);
  assert.ok(Math.abs(p.parameter-native.intersection.hits[i].parameter)<1e-10);
  assert.ok(Math.abs(p.boundaries[0].edge_parameter-native.intersection.hits[i].boundaries[0].edge_parameter)<1e-10);
 });}
 assert.equal(result.mesh.faces,4);assert.ok(Math.abs(result.mesh.volume-Math.PI*24**2*12)<1e-8);
 compareIntersection(result.mesh,native.mesh);compareIntersection(result.display_mesh,native.display_mesh);
}
for(const [offset,placement] of [[0,0],[24,0],[24+1e-9,0],[NaN,0],[14,Infinity]])assert.equal(ellipsePlanar(offset,placement).status,1);
assert.equal(ellipsePlanar(14).status,0);
console.log('Planar ellipse trims: independent roots/volume, original ellipse edge parameters, native/WASM meshes, tangency/vertex rejection and recovery passed.');

function halfEllipse(mode,offset,placement=0){
 const status=k.hagane_half_ellipse_planar_demo(mode,offset,placement);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};
}
for(const placement of [0,0.37])for(const [mode,offset] of [[0,14],[0,-14],[0,30],[1,0],[1,14],[2,0],[2,14]]){
 const {status,result}=halfEllipse(mode,offset,placement);assert.equal(status,0);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','half_ellipse_planar','--',String(mode),String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
 compareIntersection(result,native);
 const a=24*Math.sqrt(1.0625),y=24*Math.sqrt(1-(offset/a)**2);
 const expected=mode===0?(offset<0||offset>24?[]:[(34-a*Math.sqrt(1-(offset/24)**2))/2,(34+a*Math.sqrt(1-(offset/24)**2))/2]):mode===1?[17,(34+y)/2]:[(34-y)/2,17];
 assert.equal(result.intersection.hits.length,expected.length);
 result.intersection.hits.forEach((h,i)=>assert.ok(Math.abs(h.parameter-expected[i])<1e-10));
 if(mode!==0&&offset===0){const diameter=result.intersection.hits.find(h=>h.boundaries[0].coedge===1);assert.ok(Math.abs(diameter.boundaries[0].edge_parameter-0.5)<1e-12);}
 assert.equal(result.mesh.faces,4);assert.equal(result.mesh.edges,6);assert.ok(Math.abs(result.mesh.volume-Math.PI*24**2*6)<1e-8);
}
for(const args of [[3,14,0],[0,0,0],[0,1e-9,0],[0,24,0],[0,24+1e-9,0],[0,NaN,0],[1,0,Infinity]])assert.equal(halfEllipse(...args).status,1);
assert.equal(halfEllipse(1,0).status,0);
console.log('Half-ellipse planar trims: independent arc/diameter roots, original parameters, analytic volume, native/WASM parity, errors and recovery passed.');

function ellipseSegment(sweep,mode,offset,placement=0){
 const status=k.hagane_ellipse_segment_planar_demo(sweep,mode,offset,placement);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};
}
for(const sweep of [0.6,Math.PI/2,2.5,Math.PI])for(const placement of [0,0.37])for(const mode of [0,1,2]){
 const chordY=sweep===Math.PI?0:24*Math.cos(sweep/2),offset=mode===0?(24+chordY)/2:0;
 const {status,result}=ellipseSegment(sweep,mode,offset,placement);assert.equal(status,0);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','ellipse_segment_planar','--',String(sweep),String(mode),String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
 compareIntersection(result,native);
 const half=24*Math.sqrt(1.0625)*Math.sqrt(1-(offset/24)**2);
 const expected=mode===0?[(34-half)/2,(34+half)/2]:mode===1?[(34+chordY)/2,29]:[5,(34-chordY)/2];
 assert.equal(result.intersection.hits.length,2);assert.equal(result.intersection.intervals.length,1);
 result.intersection.hits.forEach((h,i)=>assert.ok(Math.abs(h.parameter-expected[i])<1e-10));
 if(mode!==0){const chord=result.intersection.hits.find(h=>h.boundaries[0].coedge===1);assert.ok(Math.abs(chord.boundaries[0].edge_parameter-0.5)<1e-12);const arc=result.intersection.hits.find(h=>h.boundaries[0].coedge===0);assert.ok(Math.abs(arc.boundaries[0].edge_parameter-sweep/2)<1e-12);}
 assert.ok(Math.abs(result.mesh.volume-24**3*(sweep-Math.sin(sweep))/4)<1e-8);
 assert.equal(result.mesh.faces,4);assert.equal(result.mesh.edges,6);
}
for(const sweep of [0,-0.1,Math.PI+0.1,2*Math.PI,1e-6,NaN,Infinity])assert.equal(ellipseSegment(sweep,0,20).status,1);
for(const [mode,offset,placement] of [[3,20,0],[0,24,0],[0,24+1e-9,0],[0,24*Math.cos(Math.PI/4),0],[0,NaN,0],[1,0,Infinity]])assert.equal(ellipseSegment(Math.PI/2,mode,offset,placement).status,1);
assert.equal(ellipseSegment(Math.PI/2,0,14).result.intersection.kind,'empty');
assert.equal(ellipseSegment(Math.PI/2,1,0).status,0);
console.log('Minor ellipse/chord trims: independent roots and volumes, angular/line provenance, native/WASM parity, contacts, unsupported sweeps and recovery passed.');

function ellipseAnnulus(offset,placement=0){
 const status=k.hagane_ellipse_annulus_planar_demo(offset,placement);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};
}
for(const placement of [0,0.37])for(const offset of [-6,6,18,30]){
 const {status,result}=ellipseAnnulus(offset,placement);assert.equal(status,0);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','ellipse_annulus_planar','--',String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 const outer=Math.sqrt(576-offset**2)*Math.sqrt(1.0625),inner=Math.sqrt(144-offset**2)*Math.sqrt(1.0625);
 const expected=offset===30?[]:Math.abs(offset)>12?[(34-outer)/2,(34+outer)/2]:[(34-outer)/2,(34-inner)/2,(34+inner)/2,(34+outer)/2];
 assert.equal(result.intersection.hits.length,expected.length);assert.equal(result.intersection.intervals.length,expected.length/2);
 result.intersection.hits.forEach((h,i)=>assert.ok(Math.abs(h.parameter-expected[i])<1e-10));
 if(Math.abs(offset)<12)assert.deepEqual(result.intersection.hits.map(h=>h.boundaries[0].wire),[0,1,1,0]);
 assert.equal(result.mesh.faces,6);assert.equal(result.mesh.edges,12);assert.equal(result.face,5);assert.ok(Math.abs(result.mesh.volume-Math.PI*(576-144)*12)<1e-8);
}
for(const [offset,placement] of [[0,0],[12,0],[12+1e-9,0],[24,0],[24+1e-9,0],[NaN,0],[6,Infinity]])assert.equal(ellipseAnnulus(offset,placement).status,1);
assert.equal(ellipseAnnulus(6).status,0);
console.log('Ellipse annulus: independent roots/volume, two material intervals, hole provenance, native/WASM parity, contacts and recovery passed.');

function eccentricEllipse(center,offset,placement=0){
 const status=k.hagane_ellipse_eccentric_planar_demo(center,offset,placement);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};
}
for(const center of [-6,0,6,10])for(const placement of [0,0.37])for(const offset of [4,14,30]){
 const {status,result}=eccentricEllipse(center,offset,placement);assert.equal(status,0);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','ellipse_eccentric_planar','--',String(center),String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 const stretch=Math.sqrt(1.0625),outer=Math.sqrt(576-offset**2)*stretch,inner=Math.sqrt(64-offset**2)*stretch,shift=center*stretch;
 const expected=offset===30?[]:offset>8?[(34-outer)/2,(34+outer)/2]:[(34-outer)/2,(34+shift-inner)/2,(34+shift+inner)/2,(34+outer)/2];
 assert.equal(result.intersection.hits.length,expected.length);assert.equal(result.intersection.intervals.length,expected.length/2);
 result.intersection.hits.forEach((h,i)=>assert.ok(Math.abs(h.parameter-expected[i])<1e-10));
 assert.ok(Math.abs(result.mesh.volume-(Math.PI*(576-64)*12+16*Math.PI*center))<1e-8);
}
for(const center of [16,16-1e-9,20,NaN,Infinity])assert.equal(eccentricEllipse(center,4).status,1);
for(const [offset,placement] of [[0,0],[8,0],[8+1e-9,0],[24,0],[NaN,0],[4,Infinity]])assert.equal(eccentricEllipse(6,offset,placement).status,1);
assert.equal(eccentricEllipse(6,4).status,0);
console.log('Eccentric ellipse holes: independent displaced roots/first-moment volume, native/WASM parity, containment/contact errors and recovery passed.');

function multiHoleEllipse(spread,offset,placement=0){
 const status=k.hagane_ellipse_multi_hole_planar_demo(spread,offset,placement);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};
}
for(const spread of [6,9,12])for(const placement of [0,0.37])for(const offset of [2,5.5,14,30]){
 const {status,result}=multiHoleEllipse(spread,offset,placement);assert.equal(status,0);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','ellipse_multi_hole_planar','--',String(spread),String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 const stretch=Math.sqrt(1.0625),outer=Math.sqrt(576-offset**2)*stretch;
 const expected=offset===30?[]:[[(34-outer)/2,0],[(34+outer)/2,0]];
 if(offset<5)for(const sign of [-1,1])expected.push([(34-spread*stretch+sign*Math.sqrt(25-offset**2)*stretch)/2,1]);
 if(offset<6)for(const sign of [-1,1])expected.push([(34+spread*stretch+sign*Math.sqrt(36-offset**2)*stretch)/2,2]);
 expected.sort((a,b)=>a[0]-b[0]);assert.equal(result.intersection.hits.length,expected.length);assert.equal(result.intersection.intervals.length,expected.length/2);
 result.intersection.hits.forEach((h,i)=>{assert.ok(Math.abs(h.parameter-expected[i][0])<1e-10);assert.equal(h.boundaries[0].wire,expected[i][1]);});
 assert.equal(result.mesh.faces,8);assert.equal(result.mesh.edges,18);assert.ok(Math.abs(result.mesh.volume-Math.PI*(6180+2.75*spread))<1e-8);
}
for(const spread of [0,5,5.5,5.5+1e-9,18,NaN,Infinity])assert.equal(multiHoleEllipse(spread,2).status,1);
for(const [offset,placement] of [[0,0],[5,0],[5+1e-9,0],[6,0],[24,0],[NaN,0],[2,Infinity]])assert.equal(multiHoleEllipse(9,offset,placement).status,1);
assert.equal(multiHoleEllipse(9,2).status,0);
console.log('Multiple ellipse holes: independent roots/first moments, one/two/three material intervals, hole wire provenance, native/WASM parity, separation/contact errors and recovery passed.');

function tiltedBore(tilt,offset,placement=0){
 const status=k.hagane_tilted_bore_demo(tilt,offset,placement);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};
}
for(const tilt of [-0.9,-0.5,0,0.5,0.9])for(const placement of [0,0.37])for(const offset of [3,14,30]){
 const {status,result}=tiltedBore(tilt,offset,placement);assert.equal(status,0);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','tilted_bore','--',String(tilt),String(offset),String(placement)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 const outer=Math.sqrt(576-offset**2),inner=Math.sqrt(36-offset**2)/Math.cos(tilt),center=8*Math.tan(tilt);
 const expected=offset===30?[]:offset>6?[(34-outer)/2,(34+outer)/2]:[(34-outer)/2,(34+center-inner)/2,(34+center+inner)/2,(34+outer)/2];
 assert.equal(result.intersection.hits.length,expected.length);assert.equal(result.intersection.intervals.length,expected.length/2);
 result.intersection.hits.forEach((h,i)=>assert.ok(Math.abs(h.parameter-expected[i])<1e-10));
 assert.equal(result.mesh.faces,6);assert.equal(result.mesh.edges,12);assert.ok(Math.abs(result.mesh.volume-Math.PI*(576-36/Math.cos(tilt))*16)<1e-8);
}
for(const tilt of [1.04,Math.PI/3+0.01,NaN,Infinity])assert.equal(tiltedBore(tilt,3).status,1);
for(const [offset,placement] of [[0,0],[6,0],[6+1e-9,0],[24,0],[NaN,0],[3,Infinity]])assert.equal(tiltedBore(0.5,offset,placement).status,1);
assert.equal(tiltedBore(0.5,3).status,0);
console.log('Unequal-axis ellipse hole/tilted bore: independent roots/volume, native/WASM parity, tilt placement, contacts/containment errors and recovery passed.');

for(const offset of [-3,0.5,3,14]){
 const status=k.hagane_separated_tilted_bores_demo(offset);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));
 assert.equal(status,0);assert.equal(result.mesh.faces,8);assert.equal(result.mesh.edges,18);
 assert.ok(Math.abs(result.mesh.volume-Math.PI*(576-18/Math.cos(1))*8)<1e-8);
 assert.equal(result.intersection.intervals.length,Math.abs(offset)===3?2:1);
 const outer=Math.sqrt(576-offset**2),inner=Math.sqrt(8)/Math.cos(1),center=4*Math.tan(1);
 const roots=Math.abs(offset)===3?[(34-outer)/2,(34+center-inner)/2,(34+center+inner)/2,(34+outer)/2]:[(34-outer)/2,(34+outer)/2];
 assert.equal(result.intersection.hits.length,roots.length);result.intersection.hits.forEach((hit,i)=>assert.ok(Math.abs(hit.parameter-roots[i])<1e-10));
 if(offset===3){const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','separated_tilted_bores'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);}
}
assert.equal(k.hagane_separated_tilted_bores_demo(NaN),1);
assert.equal(k.hagane_separated_tilted_bores_demo(1),1); // Hole tangent.
assert.equal(k.hagane_separated_tilted_bores_demo(3),0);
console.log('Supporting-line separation: parallel tilted bores, independent volume/intervals, native/WASM parity, contacts/errors and recovery passed.');

for(const control of [8,14,24]){const {status,result}=preset(26,control);assert.equal(status,0);assert.ok(Math.abs(result.volume-Math.PI*(576-2*(control/6)**2/Math.cos(1))*8)<1e-8);}
for(const control of [0,7,25,NaN,Infinity])assert.equal(preset(26,control).status,1);
assert.equal(preset(26,14,0).status,1);assert.equal(preset(26).status,0);

for(const tilt of [-1,-.7,0,.7,1])for(const offset of [-3,.5,3,14]){
 const status=k.hagane_divergent_tilted_bores_demo(tilt,offset);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));
 assert.equal(status,0);assert.equal(result.mesh.faces,8);assert.equal(result.mesh.edges,18);
 assert.ok(Math.abs(result.mesh.volume-Math.PI*(576-18/Math.cos(tilt))*8)<1e-8);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','divergent_tilted_bores','--',String(tilt),String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 const outer=Math.sqrt(576-offset**2),inner=Math.sqrt(8)/Math.cos(tilt),center=(offset>0?-1:1)*4*Math.tan(tilt);
 const roots=Math.abs(offset)===3?[(34-outer)/2,(34+center-inner)/2,(34+center+inner)/2,(34+outer)/2]:[(34-outer)/2,(34+outer)/2];
 assert.equal(result.intersection.hits.length,roots.length);result.intersection.hits.forEach((hit,i)=>assert.ok(Math.abs(hit.parameter-roots[i])<1e-10));
}
for(const [tilt,offset] of [[NaN,3],[Infinity,3],[1.1,3],[.7,NaN],[.7,1],[.7,0]])assert.equal(k.hagane_divergent_tilted_bores_demo(tilt,offset),1);
assert.equal(k.hagane_divergent_tilted_bores_demo(.7,3),0);
for(const control of [8,14,24]){const {status,result}=preset(27,control);assert.equal(status,0);assert.ok(Math.abs(result.volume-Math.PI*(576-2*(control/6)**2/Math.cos(.7))*8)<1e-8);}
for(const control of [0,7,25,NaN,Infinity])assert.equal(preset(27,control).status,1);
assert.equal(preset(27,14,0).status,1);assert.equal(preset(27).status,0);
console.log('Nonparallel tilted bores: full-height certificate, independent cap roots/volume, native/WASM parity, errors and recovery passed.');

for(const angle of [0,.4,1.2,-2.1,Math.PI]){
 assert.equal(k.hagane_oriented_bores_demo(angle,1),0);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','oriented_bores','--',String(angle)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
 assert.ok(Math.abs(result.mesh.volume-Math.PI*(784-9/Math.cos(.7)-9/Math.cos(.5))*8)<1e-8);
 const hole=result.intersection.hits.filter(hit=>hit.boundaries[0].wire===1);
 const half=Math.sqrt(8)/Math.cos(.7);
 assert.equal(hole.length,2);hole.forEach((hit,i)=>assert.ok(Math.abs(hit.parameter-(40+(i?half:-half))/2)<1e-10));
}
for(const [angle,offset] of [[NaN,1],[Infinity,1],[.4,NaN],[.4,3],[.4,0]])assert.equal(k.hagane_oriented_bores_demo(angle,offset),1);
assert.equal(k.hagane_oriented_bores_demo(.4,1),0);
for(const control of [8,14,24]){const {status,result}=preset(28,control);assert.equal(status,0);assert.ok(Math.abs(result.volume-Math.PI*(784-(control/6)**2*(1/Math.cos(.7)+1/Math.cos(.5)))*8)<1e-8);}
for(const control of [0,7,25,NaN,Infinity])assert.equal(preset(28,control).status,1);
assert.equal(preset(28,14,0).status,1);assert.equal(preset(28).status,0);
console.log('Independent bore azimuths: exact rotated ellipse roots/volume, native/WASM parity, controls/errors and recovery passed.');

for(const radius of [8,14,24])for(const depth of [4,12,20]){
 assert.equal(k.hagane_blind_bore_demo(radius,depth),0);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));
 assert.equal(result.faces,8);assert.equal(result.edges,15);assert.ok(Math.abs(result.volume-(115200-Math.PI*radius**2*depth))<1e-8);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','blind_bore','--',String(radius),String(depth)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
}
for(const [radius,depth] of [[0,16],[30,16],[NaN,16],[14,0],[14,24],[14,24-1e-9],[14,NaN],[14,Infinity]])assert.equal(k.hagane_blind_bore_demo(radius,depth),1);
assert.equal(k.hagane_blind_bore_demo(14,16),0);
for(const radius of [0,30,NaN,Infinity])assert.equal(preset(29,radius).status,1);
assert.equal(preset(29,14,0).status,1);assert.equal(preset(29).status,0);
console.log('Blind bores: exact retained floors, independent volume, depth/radius native/WASM parity, breakthroughs/errors and recovery passed.');

for(let face=0;face<6;face++)for(const depth of [4,12,20]){
 assert.equal(k.hagane_box_face_blind_bore_demo(face,7,depth),0);
 const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));
 assert.equal(result.faces,8);assert.equal(result.edges,15);assert.ok(Math.abs(result.volume-(192000-Math.PI*49*depth))<1e-8);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','box_face_blind_bore','--',String(face),'7',String(depth)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);
}
for(const [face,radius,depth] of [[6,7,20],[999,7,20],[0,0,20],[0,NaN,20],[0,7,40],[1,7,40-1e-9],[2,7,80],[5,7,60],[0,7,0],[0,7,NaN]])assert.equal(k.hagane_box_face_blind_bore_demo(face,radius,depth),1);
assert.equal(k.hagane_box_face_blind_bore_demo(5,7,20),0);
for(const radius of [0,40,NaN,Infinity])assert.equal(preset(30,radius).status,1);
assert.equal(preset(30,14,0).status,1);assert.equal(preset(30).status,0);
console.log('Six-face blind bores: side/bottom entry, independent volume/depth, native/WASM geometry parity, errors and recovery passed.');

function workflow(document){const text=typeof document==='string'?document:JSON.stringify(document);k.hagane_workflow_begin();for(const byte of new TextEncoder().encode(text)){if(k.hagane_workflow_push_byte(byte))break;}assert.equal(k.hagane_workflow_finish(),0);return JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));}
const workflowFixture=JSON.parse(readFileSync(new URL('../docs/workflow-example.json',import.meta.url),'utf8'));
const workflowFile=new URL('../target/workflow-parity.json',import.meta.url);
try{
 for(const radius of [8,14,24])for(const depth of [8,16,20]){
  const doc=structuredClone(workflowFixture);doc.operations[1].radius=radius;doc.operations[1].depth=depth;
  const report=workflow(doc);assert.equal(report.ok,true);assert.ok(Math.abs(report.mesh.volume-(115200-Math.PI*radius**2*depth))<1e-8);assert.deepEqual(workflow(report.document),report);
  writeFileSync(workflowFile,JSON.stringify(doc));const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','workflow','--',workflowFile.pathname],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(report,native);
 }
}finally{try{unlinkSync(workflowFile);}catch{}}
for(const [field,value,code] of [['radius',30,'side_clearance'],['depth',24,'floor_thickness'],['input','missing','invalid_reference']]){
 const doc=structuredClone(workflowFixture);doc.operations[1][field]=value;const report=workflow(doc);assert.equal(report.ok,false);assert.equal(report.diagnostic.code,code);assert.equal(report.diagnostic.operation_id,'bore-1');assert.equal('mesh' in report,false);
}
for(const text of ['{','{}','[]','{"schema_version":1,"schema_version":2}'])assert.equal(workflow(text).diagnostic.code,'invalid_document');
assert.equal(workflow({...workflowFixture,schema_version:99}).diagnostic.code,'unsupported_schema');
k.hagane_workflow_begin();assert.equal(k.hagane_workflow_push_byte(256),1);assert.equal(k.hagane_workflow_finish(),0);assert.equal(JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len()))).ok,false);
k.hagane_workflow_begin();k.hagane_workflow_push_byte(255);assert.equal(k.hagane_workflow_finish(),0);assert.equal(JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len()))).diagnostic.code,'invalid_document');
assert.equal(workflow(' '.repeat(65537)).ok,false);assert.equal(workflow(workflowFixture).ok,true);
console.log('Editable workflow: versioned document round trips, independent volume, native/WASM parity, operation diagnostics, bounded UTF-8 transport and recovery passed.');

const extremeWorkflow=structuredClone(workflowFixture);extremeWorkflow.operations[1].center=[Number.MAX_VALUE,0];extremeWorkflow.operations[1].radius=Number.MAX_VALUE;
assert.equal(workflow(extremeWorkflow).diagnostic.category,'numerically_unresolved');assert.equal(workflow(extremeWorkflow).diagnostic.code,'finite_clearance');

const mixedWorkflow=structuredClone(workflowFixture);mixedWorkflow.operations.push({kind:'bore',id:'bore-2',input:'bore-1',mode:'through',center:[24,0],radius:4});
const mixedFile=new URL('../target/workflow-mixed-parity.json',import.meta.url);
try {writeFileSync(mixedFile,JSON.stringify(mixedWorkflow));const nativeMixed=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','workflow','--',mixedFile.pathname],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(workflow(mixedWorkflow),nativeMixed);} finally {unlinkSync(mixedFile);}
const mixedReport=workflow(mixedWorkflow);assert.equal(mixedReport.ok,true);assert.ok(Math.abs(mixedReport.mesh.volume-(115200-Math.PI*(196*16+16*24)))<1e-8);assert.equal(mixedReport.mesh.faces,9);assert.deepEqual(workflow(mixedReport.document),mixedReport);
mixedWorkflow.operations[2].center=[18,0];const mixedFailure=workflow(mixedWorkflow);assert.equal(mixedFailure.diagnostic.code,'bore_clearance');assert.equal(mixedFailure.diagnostic.operation_id,'bore-2');assert.equal('mesh' in mixedFailure,false);
console.log('Mixed operation history: both cuts retained, analytic volume, topology, round trips and second-operation contact diagnostics passed.');

function incrementalWorkflow(document){const text=typeof document==='string'?document:JSON.stringify(document);k.hagane_workflow_begin();for(const byte of new TextEncoder().encode(text)){if(k.hagane_workflow_push_byte(byte))break;}assert.equal(k.hagane_workflow_finish_incremental(),0);return JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));}
k.hagane_workflow_reset_session();
const incrementalBase=JSON.parse(readFileSync(new URL('../docs/workflow-multiple-example.json',import.meta.url),'utf8'));
const incrementalLate=structuredClone(incrementalBase);incrementalLate.operations[2].radius=5;
const incrementalEarly=structuredClone(incrementalLate);incrementalEarly.operations[1].depth=12;
const incrementalBad=structuredClone(incrementalEarly);incrementalBad.operations[2].center=[18,0];
const incrementalStock=structuredClone(incrementalEarly);incrementalStock.operations[0].size[0]=100;
const incrementalPolicy=structuredClone(incrementalStock);incrementalPolicy.tolerance.angular*=2;
const incrementalShort=structuredClone(incrementalPolicy);incrementalShort.operations.pop();
const incrementalEdits=[incrementalBase,incrementalBase,incrementalLate,incrementalEarly,incrementalBad,incrementalEarly,incrementalStock,incrementalPolicy,incrementalShort];
const incrementalExpected=[[0,3],[3,0],[2,1],[1,2],null,[3,0],[0,3],[0,3],[2,0]];
const incrementalReports=incrementalEdits.map((doc,i)=>{const report=incrementalWorkflow(doc);if(incrementalExpected[i]){assert.equal(report.ok,true);assert.deepEqual([report.rebuild.reused_operations,report.rebuild.rebuilt_operations],incrementalExpected[i]);const fresh=workflow(doc);assert.deepEqual(report.mesh,fresh.mesh);assert.deepEqual(report.document,fresh.document);}else{assert.equal(report.ok,false);assert.equal(report.diagnostic.code,'bore_clearance');assert.equal('rebuild' in report,false);}return report;});
const incrementalFiles=incrementalEdits.map((_,i)=>new URL(`../target/workflow-incremental-${i}.json`,import.meta.url));
try {
 incrementalFiles.forEach((file,i)=>writeFileSync(file,JSON.stringify(incrementalEdits[i])));
 const nativeReports=execFileSync('cargo',['run','--quiet','--locked','--example','workflow_session','--',...incrementalFiles.map(f=>f.pathname)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}).trim().split('\n').map(JSON.parse);
 assert.equal(nativeReports.length,incrementalReports.length);nativeReports.forEach((report,i)=>compareIntersection(report,incrementalReports[i]));
} finally {incrementalFiles.forEach(file=>{try{unlinkSync(file);}catch{}});}
assert.equal(incrementalWorkflow('{').ok,false);assert.equal(incrementalWorkflow(' '.repeat(65537)).ok,false);
k.hagane_workflow_begin();k.hagane_workflow_push_byte(255);assert.equal(k.hagane_workflow_finish_incremental(),0);assert.equal(JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len()))).ok,false);
assert.equal(incrementalWorkflow(incrementalShort).rebuild.rebuilt_operations,0);
k.hagane_workflow_reset_session();assert.equal(incrementalWorkflow(incrementalShort).rebuild.rebuilt_operations,2);
console.log('Incremental history: actual prefix reuse, changed suffixes, policy invalidation, truncation, failure recovery, reset and native/WASM session parity passed.');

const extrusionDocument=JSON.parse(readFileSync(new URL('../docs/workflow-extrusion-example.json',import.meta.url),'utf8'));
const extrusionReport=workflow(extrusionDocument);assert.equal(extrusionReport.ok,true);assert.ok(Math.abs(extrusionReport.mesh.volume-(4600*24-Math.PI*(196*16+16*24)))<1e-8);assert.equal(extrusionReport.mesh.faces,13);
const extrusionFile=new URL('../target/workflow-extrusion-parity.json',import.meta.url);
try {writeFileSync(extrusionFile,JSON.stringify(extrusionDocument));compareIntersection(extrusionReport,JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','workflow','--',extrusionFile.pathname],{encoding:'utf8',cwd:new URL('../',import.meta.url)})));}finally{try{unlinkSync(extrusionFile);}catch{}}
k.hagane_workflow_reset_session();assert.equal(incrementalWorkflow(extrusionDocument).rebuild.rebuilt_operations,3);assert.equal(incrementalWorkflow(extrusionDocument).rebuild.rebuilt_operations,0);
const heightEdit=structuredClone(extrusionDocument);heightEdit.operations[0].height=32;assert.equal(incrementalWorkflow(heightEdit).rebuild.reused_operations,0);assert.ok(Math.abs(workflow(heightEdit).mesh.volume-(4600*32-Math.PI*(196*16+16*32)))<1e-8);
const exteriorTool=structuredClone(heightEdit);exteriorTool.operations[1].center=[38,28];const exteriorReport=incrementalWorkflow(exteriorTool);assert.equal(exteriorReport.diagnostic.code,'side_clearance');assert.equal('mesh' in exteriorReport,false);assert.equal(incrementalWorkflow(heightEdit).rebuild.rebuilt_operations,0);
const brokenProfile=structuredClone(heightEdit);brokenProfile.operations[0].outer[1]=brokenProfile.operations[0].outer[0];assert.equal(workflow(brokenProfile).diagnostic.code,'profile_rejected');
console.log('Polygon extrusion history: exact mixed cuts, independent volume, native/WASM parity, incremental height edits, outside-profile rejection and cache recovery passed.');

const openingsDocument=JSON.parse(readFileSync(new URL('../docs/workflow-extrusion-openings-example.json',import.meta.url),'utf8'));
const openingsReport=workflow(openingsDocument);assert.equal(openingsReport.ok,true);assert.ok(Math.abs(openingsReport.mesh.volume-((4600-192)*24-Math.PI*(196*16+16*24)))<1e-8);assert.equal(openingsReport.mesh.faces,17);
const openingsFile=new URL('../target/workflow-openings-parity.json',import.meta.url);
try{writeFileSync(openingsFile,JSON.stringify(openingsDocument));compareIntersection(openingsReport,JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','workflow','--',openingsFile.pathname],{encoding:'utf8',cwd:new URL('../',import.meta.url)})));}finally{try{unlinkSync(openingsFile);}catch{}}
k.hagane_workflow_reset_session();assert.equal(incrementalWorkflow(openingsDocument).rebuild.rebuilt_operations,3);
const openingContact=structuredClone(openingsDocument);openingContact.operations[1].center=[-8,0];const contactReport=incrementalWorkflow(openingContact);assert.equal(contactReport.diagnostic.code,'profile_hole_clearance');assert.equal(contactReport.diagnostic.operation_id,'bore-1');assert.equal(contactReport.diagnostic.measured_clearance,0);assert.equal('mesh' in contactReport,false);assert.equal(incrementalWorkflow(openingsDocument).rebuild.rebuilt_operations,0);
const openingHeight=structuredClone(openingsDocument);openingHeight.operations[0].height=32;assert.equal(incrementalWorkflow(openingHeight).rebuild.reused_operations,0);assert.ok(Math.abs(workflow(openingHeight).mesh.volume-((4600-192)*32-Math.PI*(196*16+16*32)))<1e-8);
const duplicateOpening=structuredClone(openingsDocument);duplicateOpening.operations[0].holes.push(duplicateOpening.operations[0].holes[0]);assert.equal(workflow(duplicateOpening).diagnostic.code,'profile_rejected');
console.log('Polygon profile openings: independent volume, native/WASM parity, closed topology, contact rejection, changed stock invalidation and cache recovery passed.');

const skewDocument=JSON.parse(readFileSync(new URL('../docs/workflow-skew-extrusion-example.json',import.meta.url),'utf8'));const skewReport=workflow(skewDocument);assert.equal(skewReport.ok,true);assert.equal(skewReport.mesh.faces,14);assert.ok(Math.abs(skewReport.mesh.volume-4408*24)<1e-8);
const skewFile=new URL('../target/workflow-skew-parity.json',import.meta.url);try{writeFileSync(skewFile,JSON.stringify(skewDocument));compareIntersection(skewReport,JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','workflow','--',skewFile.pathname],{encoding:'utf8',cwd:new URL('../',import.meta.url)})));}finally{try{unlinkSync(skewFile);}catch{}}
k.hagane_workflow_reset_session();assert.equal(incrementalWorkflow(skewDocument).rebuild.rebuilt_operations,1);const skewEdit=structuredClone(skewDocument);skewEdit.operations[0].offset=[24,-8];assert.equal(incrementalWorkflow(skewEdit).rebuild.reused_operations,0);assert.ok(Math.abs(workflow(skewEdit).mesh.volume-4408*24)<1e-8);
const skewTool=structuredClone(skewEdit);skewTool.operations.push({kind:'bore',id:'bore-1',input:'extrusion-1',mode:'through',center:[24,0],radius:4});
const skewCut=incrementalWorkflow(skewTool);assert.equal(skewCut.ok,true);assert.equal(skewCut.rebuild.reused_operations,1);assert.ok(Math.abs(skewCut.mesh.volume-(4408-16*Math.PI)*24)<1e-8);
try{writeFileSync(skewFile,JSON.stringify(skewTool));compareIntersection(workflow(skewTool),JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','workflow','--',skewFile.pathname],{encoding:'utf8',cwd:new URL('../',import.meta.url)})));}finally{try{unlinkSync(skewFile);}catch{}}
const skewBlind=structuredClone(skewTool);skewBlind.operations[1].mode='blind';skewBlind.operations[1].depth=8;const skewBlindReport=incrementalWorkflow(skewBlind);assert.equal(skewBlindReport.ok,true);assert.ok(Math.abs(skewBlindReport.mesh.volume-(4408*24-16*Math.PI*8))<1e-8);try{writeFileSync(skewFile,JSON.stringify(skewBlind));compareIntersection(workflow(skewBlind),JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','workflow','--',skewFile.pathname],{encoding:'utf8',cwd:new URL('../',import.meta.url)})));}finally{try{unlinkSync(skewFile);}catch{}}skewBlind.operations[1].depth=24;const skewFailure=incrementalWorkflow(skewBlind);assert.equal(skewFailure.diagnostic.code,'floor_thickness');assert.equal('mesh' in skewFailure,false);assert.equal(incrementalWorkflow(skewTool).rebuild.reused_operations,1);
const crossing=structuredClone(skewTool);Object.assign(crossing.operations[0],{outer:[[-40,-30],[40,-30],[40,30],[-40,30]],holes:[[[-2,-2],[2,-2],[2,2],[-2,2]]],offset:[20,0]});Object.assign(crossing.operations[1],{center:[10,0],radius:1});assert.equal(incrementalWorkflow(crossing).diagnostic.code,'profile_hole_clearance');assert.equal(incrementalWorkflow(skewTool).rebuild.rebuilt_operations,0);
console.log('Skew extrusion history: exact through-bore volume, native/WASM geometry parity, incremental reuse, swept-opening rejection, blind volume, breakthrough rejection and cache recovery passed.');

const bottomDocument=JSON.parse(readFileSync(new URL('../docs/workflow-bottom-blind-example.json',import.meta.url),'utf8'));
const bottomReport=workflow(bottomDocument);assert.equal(bottomReport.ok,true);assert.ok(Math.abs(bottomReport.mesh.volume-(4408*24-16*Math.PI*8))<1e-8);
try{writeFileSync(skewFile,JSON.stringify(bottomDocument));compareIntersection(bottomReport,JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','workflow','--',skewFile.pathname],{encoding:'utf8',cwd:new URL('../',import.meta.url)})));}finally{try{unlinkSync(skewFile);}catch{}}
k.hagane_workflow_reset_session();assert.equal(incrementalWorkflow(bottomDocument).rebuild.rebuilt_operations,2);const topEdit=structuredClone(bottomDocument);delete topEdit.operations[1].entry;assert.equal(incrementalWorkflow(topEdit).rebuild.reused_operations,1);
const bottomInvalid=structuredClone(bottomDocument);bottomInvalid.operations[1].depth=24;assert.equal(incrementalWorkflow(bottomInvalid).diagnostic.code,'floor_thickness');assert.equal(incrementalWorkflow(topEdit).rebuild.rebuilt_operations,0);
console.log('Bottom-entry blind workflow: analytic volume, native/WASM full mesh parity, entry edits, prefix reuse and failure recovery passed.');

const opposingDocument=JSON.parse(readFileSync(new URL('../docs/workflow-opposing-blind-example.json',import.meta.url),'utf8'));
const opposingReport=workflow(opposingDocument);assert.equal(opposingReport.ok,true);assert.ok(Math.abs(opposingReport.mesh.volume-(4408*24-Math.PI*(16*8+36*10)))<1e-8);
try{writeFileSync(skewFile,JSON.stringify(opposingDocument));compareIntersection(opposingReport,JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','workflow','--',skewFile.pathname],{encoding:'utf8',cwd:new URL('../',import.meta.url)})));}finally{try{unlinkSync(skewFile);}catch{}}
k.hagane_workflow_reset_session();incrementalWorkflow(opposingDocument);const opposingEdit=structuredClone(opposingDocument);opposingEdit.operations[2].depth=9;assert.equal(incrementalWorkflow(opposingEdit).rebuild.reused_operations,2);const opposingInvalid=structuredClone(opposingEdit);opposingInvalid.operations[2].depth=16;const webFailure=incrementalWorkflow(opposingInvalid);assert.equal(webFailure.diagnostic.code,'bore_web_thickness');assert.equal(webFailure.diagnostic.measured_clearance,0);assert.equal('mesh' in webFailure,false);assert.equal(incrementalWorkflow(opposingEdit).rebuild.rebuilt_operations,0);
console.log('Opposing blind cuts: exact web/volume, native/WASM report and mesh parity, depth-prefix reuse, touching-web rejection and recovery passed.');

function planarBoolean(mode,offset){const status=k.hagane_planar_convex_boolean_demo(mode,offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const mode of [0,1])for(const offset of [0,5,60]){const {status,result}=planarBoolean(mode,offset);assert.equal(status,0);const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','planar_convex_boolean','--',String(mode),String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);if(offset===0)assert.ok(Math.abs(result.mesh.volume-(mode===0?105120:2400))<1e-8);}
assert.notEqual(planarBoolean(0,10).status,0);assert.notEqual(planarBoolean(2,0).status,0);assert.equal(planarBoolean(0,0).status,0);
console.log('Planar subject/convex tool: repeated holes, analytic volume, native/WASM full mesh parity, empty/unchanged results, contacts and recovery passed.');

function splitComponents(offset){const status=k.hagane_split_components_demo(offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const offset of [0,1,-10]){const {status,result}=splitComponents(offset);assert.equal(status,0);const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','split_components','--',String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);assert.equal(result.negative.length,1);assert.equal(result.positive.length,offset===-10?1:2);assert.ok(Math.abs([...result.negative,...result.positive].reduce((s,m)=>s+m.volume,0)-45600)<1e-8);}
assert.notEqual(splitComponents(-5).status,0);assert.notEqual(splitComponents(30).status,0);assert.equal(splitComponents(0).status,0);console.log('Multi-component plane partition: independent closed solids, conserved volume, native/WASM full mesh parity, contacts and recovery passed.');

function componentBoolean(mode,offset){const status=k.hagane_component_boolean_demo(mode,offset);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const mode of [0,1])for(const offset of [0,5,60]){const {status,result}=componentBoolean(mode,offset);assert.equal(status,0);const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','component_boolean','--',String(mode),String(offset)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result,native);assert.equal(result.components.length,offset===60?(mode===0?1:0):2);assert.ok(Math.abs(result.volume-(mode===0?(offset===60?57600:49920):(offset===60?0:9600)))<1e-8);}
assert.notEqual(componentBoolean(0,26).status,0);assert.notEqual(componentBoolean(0,26-5e-8).status,0);assert.notEqual(componentBoolean(2,0).status,0);assert.equal(componentBoolean(0,0).status,0);console.log('Multi-component Booleans: two-part difference/intersection, conserved analytic volumes, native/WASM full mesh parity, empty/unchanged results, contacts and recovery passed.');

function stepExport(document){k.hagane_workflow_begin();for(const byte of new TextEncoder().encode(typeof document==='string'?document:JSON.stringify(document)))if(k.hagane_workflow_push_byte(byte))break;const status=k.hagane_workflow_finish_step_export();return {status,result:JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())))};}
k.hagane_workflow_reset_session();incrementalWorkflow(skewDocument);
const exportedStep=stepExport(skewDocument);assert.equal(exportedStep.status,0);assert.equal(exportedStep.result.units,'mm');assert.equal(exportedStep.result.step,execFileSync('cargo',['run','--quiet','--locked','--example','step_export'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
for(const fixture of ['workflow-skew-bores-example.json','workflow-bottom-blind-example.json','workflow-opposing-blind-example.json']) {
 const document=JSON.parse(readFileSync(new URL('../docs/'+fixture,import.meta.url),'utf8'));const result=stepExport(document);assert.equal(result.status,0);assert.equal(result.result.step,execFileSync('cargo',['run','--quiet','--locked','--example','step_export','--','docs/'+fixture],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.ok(result.result.step.includes('SEAM_CURVE'));assert.ok(result.result.step.includes('PCURVE'));
}
const rejectedStep=structuredClone(opposingDocument);rejectedStep.operations[2].depth=16;assert.notEqual(stepExport(rejectedStep).status,0);assert.notEqual(stepExport('{').status,0);assert.notEqual(stepExport(' '.repeat(65537)).status,0);
k.hagane_workflow_begin();k.hagane_workflow_push_byte(255);assert.notEqual(k.hagane_workflow_finish_step_export(),0);
assert.equal(incrementalWorkflow(skewDocument).rebuild.rebuilt_operations,0);
console.log('STEP: native/WASM byte parity, mm units, through/blind cylindrical seams, invalid/input rejection and accepted-session preservation passed.');

function stepImport(text){k.hagane_step_import_begin();for(const byte of new TextEncoder().encode(text))if(k.hagane_step_import_push_byte(byte))break;const status=k.hagane_step_import_finish();return {status,result:JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())))};}
const tetraInput=readFileSync(new URL('../docs/step-tetrahedron-metres.step',import.meta.url),'utf8');const wasmTetra=stepImport(tetraInput);assert.equal(wasmTetra.status,0);const nativeTetra=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','step_import'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(wasmTetra.result,nativeTetra);assert.equal(wasmTetra.result.step,nativeTetra.step);assert.equal(stepImport(wasmTetra.result.step).status,0);
assert.notEqual(stepImport(tetraInput.replace('#31,.F.)','#31,.T.)')).status,0);assert.notEqual(stepImport(stepExport(opposingDocument).result.step.replace('CYLINDRICAL_SURFACE','CONICAL_SURFACE')).status,0);assert.notEqual(stepImport(' '.repeat(1048577)).status,0);k.hagane_step_import_begin();k.hagane_step_import_push_byte(255);assert.notEqual(k.hagane_step_import_finish(),0);k.hagane_step_import_begin();assert.equal(k.hagane_step_import_push_byte(256),1);assert.notEqual(k.hagane_step_import_finish(),0);assert.equal(stepImport(tetraInput).status,0);assert.equal(incrementalWorkflow(skewDocument).rebuild.rebuilt_operations,0);
console.log('STEP import: independent metre fixture, native/WASM full report and export parity, mm round trip, orientation/curve/size/UTF-8 errors, recovery and workflow-cache isolation passed.');

const prismInput=readFileSync(new URL('../docs/step-prism-example.step',import.meta.url),'utf8');const wasmPrism=stepImport(prismInput);assert.equal(wasmPrism.status,0);const nativePrism=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','step_import','--','docs/step-prism-example.step'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(wasmPrism.result,nativePrism);assert.equal(wasmPrism.result.step,nativePrism.step);assert.equal(wasmPrism.result.vertices,24);assert.equal(wasmPrism.result.mesh.faces,14);assert.ok(Math.abs(wasmPrism.result.mesh.volume-105792)<1e-8);assert.equal(stepImport(wasmPrism.result.step).status,0);assert.notEqual(stepImport(prismInput.replaceAll('FACE_OUTER_BOUND','FACE_BOUND')).status,0);assert.equal(stepImport(prismInput).status,0);assert.equal(incrementalWorkflow(skewDocument).rebuild.rebuilt_operations,0);
console.log('STEP prism import: polygon opening, exact volume/topology, native/WASM report and byte parity, rejected outer bounds and session isolation passed.');

for(const filename of ['step-cylinder-metres.step','step-tube-example.step']){const input=readFileSync(new URL('../docs/'+filename,import.meta.url),'utf8');const result=stepImport(input);assert.equal(result.status,0);const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','step_import','--','docs/'+filename],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(result.result,native);assert.equal(result.result.step,native.step);assert.equal(stepImport(result.result.step).status,0);assert.notEqual(stepImport(input.replace('6.283185307179586,0.','6.283185307,0.')).status,0);assert.equal(stepImport(input).status,0);}
assert.equal(incrementalWorkflow(skewDocument).rebuild.rebuilt_operations,0);
console.log('Circular STEP import: independent metre cylinder and rotated tube, native/WASM full report/byte parity, corrupt UV rejection, round trip and accepted-workflow isolation passed.');

const boredPrismInput=readFileSync(new URL('../docs/step-bored-prism-example.step',import.meta.url),'utf8');const wasmBoredPrism=stepImport(boredPrismInput);assert.equal(wasmBoredPrism.status,0);const nativeBoredPrism=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','step_import','--','docs/step-bored-prism-example.step'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(wasmBoredPrism.result,nativeBoredPrism);assert.equal(wasmBoredPrism.result.step,nativeBoredPrism.step);assert.equal(wasmBoredPrism.result.vertices,26);assert.equal(wasmBoredPrism.result.mesh.faces,15);assert.ok(Math.abs(wasmBoredPrism.result.mesh.volume-(105792-Math.PI*16*24))<1e-8);assert.equal(stepImport(wasmBoredPrism.result.step).status,0);assert.notEqual(stepImport(boredPrismInput.replace('CYLINDRICAL_SURFACE','CONICAL_SURFACE')).status,0);assert.equal(stepImport(boredPrismInput).status,0);assert.equal(incrementalWorkflow(skewDocument).rebuild.rebuilt_operations,0);
console.log('Bored prism STEP import: skew stock and polygon opening, exact through-bore volume, full native/WASM mesh/byte parity, unsupported surface rejection, round trip and workflow isolation passed.');

const blindPrismInput=readFileSync(new URL('../docs/step-blind-prism-example.step',import.meta.url),'utf8');const wasmBlindPrism=stepImport(blindPrismInput);assert.equal(wasmBlindPrism.status,0);
const nativeBlindPrism=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','step_import','--','docs/step-blind-prism-example.step'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(wasmBlindPrism.result,nativeBlindPrism);assert.equal(wasmBlindPrism.result.step,nativeBlindPrism.step);assert.equal(wasmBlindPrism.result.vertices,28);assert.equal(wasmBlindPrism.result.mesh.faces,18);assert.ok(Math.abs(wasmBlindPrism.result.mesh.volume-(105792-Math.PI*(16*8+36*10)))<1e-8);assert.equal(stepImport(wasmBlindPrism.result.step).status,0);
const diskFace=blindPrismInput.match(/ADVANCED_FACE\('',\(#\d+\),#\d+,\.[TF]\.\)/g).at(-1);assert.ok(diskFace);const badFloor=blindPrismInput.replace(diskFace,diskFace.replace(/\.[TF]\./,m=>m==='.T.'?'.F.':'.T.'));assert.notEqual(badFloor,blindPrismInput);assert.notEqual(stepImport(badFloor).status,0);assert.equal(stepImport(blindPrismInput).status,0);
for(const doc of [bottomDocument,topEdit]){const source=stepExport(doc);assert.equal(source.status,0);const imported=stepImport(source.result.step);assert.equal(imported.status,0);assert.ok(Math.abs(imported.result.mesh.volume-(105792-Math.PI*16*8))<1e-8);assert.equal(imported.result.mesh.faces,16);assert.equal(stepImport(imported.result.step).status,0);}
assert.equal(incrementalWorkflow(skewDocument).rebuild.rebuilt_operations,0);
console.log('Blind STEP import: exact opposing floors/web, bottom/top entry, native/WASM mesh and byte parity, reversed floor rejection, round trip and workflow isolation passed.');

const farInput=readFileSync(new URL('../docs/step-far-cylinder-example.step',import.meta.url),'utf8');const farResult=stepImport(farInput);assert.equal(farResult.status,0);
const nativeFar=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','step_import','--','docs/step-far-cylinder-example.step'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));compareIntersection(farResult.result,nativeFar);assert.equal(farResult.result.step,nativeFar.step);assert.equal(farResult.result.mesh.positions.length,nativeFar.mesh.positions.length);farResult.result.mesh.positions.forEach((p,i)=>assert.equal(p,nativeFar.mesh.positions[i]));assert.ok(Math.abs(farResult.result.mesh.volume-Math.PI*64*24)<1e-9);assert.equal(stepImport(farResult.result.step).status,0);
// Check wall chord midpoints independently in local coordinates, without adding two large world values.
const positions=farResult.result.mesh.positions,normals=farResult.result.mesh.normals,center=[1e12,-1e12,1e12];
for(let i=0;i<positions.length;i+=9){if(Math.abs(normals[i+2])>.5)continue;for(let k=0;k<3;k++){const a=i+3*k,b=i+3*((k+1)%3);const xy=[0,1].map(j=>positions[a+j]-center[j]+(positions[b+j]-positions[a+j])*.5);assert.ok(Math.abs(Math.hypot(...xy)-8)<=.05);}}
const precisionNative=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','placement_precision'],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));assert.ok(precisionNative.coordinate_allowance>.007&&precisionNative.coordinate_allowance<.008);assert.ok(Math.abs(precisionNative.mesh_volume-precisionNative.exact_volume)<2*Math.PI*8*24*.05);
const unresolvedInput=readFileSync(new URL('../docs/step-unresolved-display.step',import.meta.url),'utf8');const unresolved=stepImport(unresolvedInput);assert.notEqual(unresolved.status,0);assert.match(unresolved.result.error,/coordinate precision/);
const collapsedInput=unresolvedInput.replaceAll('1000000000000000.1','1000000000000000.').replaceAll(',0.125)',',0.01)');assert.notEqual(collapsedInput,unresolvedInput);const collapsed=stepImport(collapsedInput);assert.notEqual(collapsed.status,0);assert.match(collapsed.result.error,/zero-angle seam/);assert.equal(stepImport(farInput).status,0);assert.equal(incrementalWorkflow(skewDocument).rebuild.rebuilt_operations,0);
console.log('Placement precision: far STEP full native/WASM parity, independent chord bounds, stable mesh volume, unresolved display/collapsed circle rejection and recovery passed.');
