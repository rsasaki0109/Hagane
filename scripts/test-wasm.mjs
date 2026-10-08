import {readFileSync} from 'node:fs';
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
const expected=[{volume:115200-Math.PI*196*24,faces:7,edges:15},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:69336,faces:12,edges:30},{volume:Math.PI*(900-196)*24,faces:4,edges:6},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:(4800-(4-Math.PI)*196)*24,faces:10,edges:24},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42},{volume:115200-Math.PI*49*24,faces:8,edges:18},{volume:(4800-(4-Math.PI)*576)*24,faces:13,edges:31},{volume:103680,faces:15,edges:45},{volume:5376*Math.PI,faces:11,edges:26},{volume:115200-3456*Math.PI,faces:13,edges:34},{volume:115200,faces:6,edges:13},{volume:67200,faces:6,edges:12},{volume:(4096-(32*Math.SQRT2-38)**2-(32*Math.SQRT2-42)**2-2*(32*Math.SQRT2-30)**2)*24,faces:10,edges:24},{volume:107520,faces:20,edges:44},{volume:174720,faces:24,edges:52},{volume:122880,faces:26,edges:52},{volume:122880,faces:10,edges:34},{volume:174720,faces:12,edges:38},{volume:122880,faces:10,edges:24},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42},{volume:(64*48-(4-Math.PI)*100-(20*16-(4-Math.PI)*16))*24,faces:22,edges:58},{volume:(64*48-(4-Math.PI)*100-(20*16-(4-Math.PI)*16))*24,faces:34,edges:80}];
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
console.log('All 25 presets: native/WASM geometry parity, metrics, errors and recovery passed.');
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
function surface(height,weight,u,v){const status=k.hagane_generate_surface(height,weight,u,v);const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));return {status,result};}
for(const [height,weight,u,v] of [[0,1,0.5,0.5],[35,1,0.5,0.5],[35,2,0.3,0.7],[-35,0.5,0.2,0.4],[60,4,0,1],[-60,0.2,1,0]]){
 const {status,result}=surface(height,weight,u,v);assert.equal(status,0);assert.equal(result.positions.length,24*24*2*9);assert.equal(result.positions.length,result.normals.length);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs_surface','--',...[height,weight,u,v].map(String)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
 for(const field of ['point','du','dv','normal','positions','normals']){assert.equal(result[field].length,native[field].length);native[field].forEach((value,i)=>assert.ok(Math.abs(value-result[field][i])<1e-10));}
 assert.ok(Math.abs(Math.hypot(...result.normal)-1)<1e-13);const dot=(a,b)=>a.reduce((s,x,i)=>s+x*b[i],0);assert.ok(Math.abs(dot(result.du,result.normal))<1e-10);assert.ok(Math.abs(dot(result.dv,result.normal))<1e-10);
 if(height===0){assert.ok(Math.abs(result.point[0]-(80*u-40))<1e-12);assert.ok(Math.abs(result.point[1]-(60*v-30))<1e-12);assert.equal(result.point[2],0);assert.deepEqual(result.normal,[0,0,1]);}
 if(height===35&&weight===1&&u===0.5){assert.ok(Math.abs(result.point[2]-8.75)<1e-13);}
}
for(const args of [[0,0,0.5,0.5],[101,1,0.5,0.5],[NaN,1,0.5,0.5],[0,1,1.1,0.5],[0,1,0.5,-0.1]]){assert.equal(surface(...args).status,1);}
assert.equal(surface(35,1,0.5,0.5).status,0);
console.log('NURBS surfaces: 6 native/WASM grid/partial/normal parity cases, analytic fixtures and errors/recovery passed.');

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
