// Optional independent interoperability check; no OCCT code is shipped by Hagane.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
const modulePath=process.argv[2];
if(!modulePath)throw new Error('Pass an external occt-import-js installation path; see docs/step-export.md.');
const importer=await createRequire(import.meta.url)(resolve(modulePath))();
const original=JSON.parse(readFileSync(new URL('../docs/workflow-skew-extrusion-example.json',import.meta.url),'utf8'));
const reversed=structuredClone(original);reversed.operations[0].outer.reverse();for(const hole of reversed.operations[0].holes)hole.reverse();
const temporary=mkdtempSync(join(tmpdir(),'hagane-step-'));
try{
 for(const document of [original,reversed]){
  const input=join(temporary,'workflow.json');writeFileSync(input,JSON.stringify(document));
  const step=execFileSync('cargo',['run','--quiet','--locked','--example','step_export','--',input],{cwd:new URL('../',import.meta.url)});
  for(const [linearUnit,scale] of [['millimeter',1],['meter',0.001]]){
   const result=importer.ReadStepFile(step,{linearUnit,linearDeflectionType:'absolute_value',linearDeflection:0.01*scale});
   assert.equal(result.success,true);assert.equal(result.meshes.length,1);
   const mesh=result.meshes[0];assert.equal(mesh.brep_faces.length,14);
   const p=mesh.attributes.position.array;
   const bounds=[0,1,2].map(axis=>{const coordinates=p.filter((_,i)=>i%3===axis);return [Math.min(...coordinates),Math.max(...coordinates)];});
   const expected=[[-40,58],[-42,30],[-12,12]];
   for(let axis=0;axis<3;axis++)for(let side=0;side<2;side++)assert.ok(Math.abs(bounds[axis][side]-expected[axis][side]*scale)<1e-5*scale);
   let volume=0;for(let i=0;i<mesh.index.array.length;i+=3){const [a,b,c]=mesh.index.array.slice(i,i+3).map(j=>p.slice(j*3,j*3+3));volume+=(a[0]*(b[1]*c[2]-b[2]*c[1])+a[1]*(b[2]*c[0]-b[0]*c[2])+a[2]*(b[0]*c[1]-b[1]*c[0]))/6;}
   assert.ok(Math.abs(volume-105792*scale**3)<0.01*scale**3);
  }
 }
 console.log('External STEP import: skew stock, polygon opening, reversed input winding, 14 faces, bounds, signed volume and mm/metre conversion passed.');
}finally{rmSync(temporary,{recursive:true,force:true});}
