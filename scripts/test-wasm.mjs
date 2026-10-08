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
const expected=[{volume:115200-Math.PI*196*24,faces:7,edges:15},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:69336,faces:12,edges:30},{volume:Math.PI*(900-196)*24,faces:4,edges:6},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:(4800-(4-Math.PI)*196)*24,faces:10,edges:24},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42},{volume:115200-Math.PI*49*24,faces:8,edges:18},{volume:(4800-(4-Math.PI)*576)*24,faces:13,edges:31},{volume:103680,faces:15,edges:45}];
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
console.log('All 10 presets: native/WASM geometry parity, metrics, errors and recovery passed.');
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
