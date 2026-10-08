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
const expected=[{volume:115200-Math.PI*196*24,faces:7,edges:15},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:69336,faces:12,edges:30},{volume:Math.PI*(900-196)*24,faces:4,edges:6}];
for(let id=0;id<4;id++){
 const {status,result:wasm}=preset(id);assert.equal(status,0);assert.ok(Math.abs(wasm.volume-expected[id].volume)<1e-8);assert.equal(wasm.faces,expected[id].faces);assert.equal(wasm.edges,expected[id].edges);
 const native=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','part','--',String(id)],{encoding:'utf8',cwd:new URL('../',import.meta.url)}));
 assert.ok(Math.abs(native.volume-wasm.volume)<1e-9);
 for(const field of ['positions','normals']){assert.equal(native[field].length,wasm[field].length);native[field].forEach((v,i)=>assert.ok(Math.abs(v-wasm[field][i])<1e-10));}
}
assert.equal(preset(999).status,1);assert.equal(preset(1,30).status,1);assert.equal(preset(3,30).status,1);assert.equal(preset(2,14,0).status,1);assert.equal(preset(1).status,0);
console.log('All 4 presets: native/WASM geometry parity, metrics, errors and recovery passed.');
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
