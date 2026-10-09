// Optional independent STEP reader. OCCT is used only by this external checker.
// Its returned triangles provide bounded mesh volume evidence, not exact B-rep mass.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
import {resolve} from 'node:path';
const path=process.argv[2];if(!path)throw new Error('Pass an external occt-import-js installation path.');
const reader=await createRequire(import.meta.url)(resolve(path))();
const cases=[
 {args:[80,60,20,30,.2,0,0,0,0,0,1,0,1,.35,.65,.35,.65],holed:false},
 {args:[20,12,3,-2,.1,-.7,-120,30,90,.2,.8,.1,.7,.35,.5,.25,.4],holed:false},
 {args:[80,60,20,30,.2,0,0,0,0,0,1,0,1,.35,.65,.35,.65],holed:true},
 {args:[20,12,3,-2,.1,-.7,-120,30,90,.2,.8,.1,.7,.35,.5,.25,.4],holed:true},
];
const primitive=x=>x*x/2-x*x*x/3;
function crossings(points,indices,x,y){const z=[];for(let i=0;i<indices.length;i+=3){const [a,b,c]=indices.slice(i,i+3).map(id=>points[id]),det=(b[0]-a[0])*(c[1]-a[1])-(c[0]-a[0])*(b[1]-a[1]);if(Math.abs(det)<1e-12)continue;const u=((x-a[0])*(c[1]-a[1])-(y-a[1])*(c[0]-a[0]))/det,v=((b[0]-a[0])*(y-a[1])-(b[1]-a[1])*(x-a[0]))/det;if(u>=-1e-8&&v>=-1e-8&&u+v<=1+1e-8)z.push(a[2]+u*(b[2]-a[2])+v*(c[2]-a[2]));}return z.sort((a,b)=>a-b).filter((x,i,all)=>i===0||Math.abs(x-all[i-1])>1e-5);}
for(const {args,holed} of cases){const [L,W,H,b,,angle,tx,ty,tz,u0,u1,v0,v1,hu0,hu1,hv0,hv1]=args,r=[[u0,u1],[v0,v1]],hole=[[hu0,hu1],[hv0,hv1]],V=q=>L*W*(H*(q[0][1]-q[0][0])*(q[1][1]-q[1][0])+4*b*(primitive(q[0][1])-primitive(q[0][0]))*(primitive(q[1][1])-primitive(q[1][0]))),expected=V(r)-(holed?V(hole):0),faces=holed?10:6;
 const report=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs_graph_step','--',...args.map(String),String(Number(holed))],{encoding:'utf8',maxBuffer:16*1024*1024,cwd:new URL('../',import.meta.url)})),step=report.step;assert.equal((step.match(/B_SPLINE_SURFACE_WITH_KNOTS\(/g)??[]).length,faces);assert.equal((step.match(/ADVANCED_FACE\(/g)??[]).length,faces);assert.ok(step.includes('MANIFOLD_SOLID_BREP'));assert.ok(!step.includes('TRIANGULATED_FACE_SET'));
 for(const [linearUnit,scale] of [['millimeter',1],['meter',.001]]){const deflection=1e-4*scale,result=reader.ReadStepFile(Buffer.from(step),{linearUnit,linearDeflectionType:'absolute_value',linearDeflection:deflection});assert.equal(result.success,true);assert.equal(result.meshes.length,1);const mesh=result.meshes[0];assert.equal(mesh.brep_faces.length,faces);const array=mesh.attributes.position.array,points=Array.from({length:array.length/3},(_,i)=>array.slice(i*3,i*3+3)),index=mesh.index.array;
 const nodeIds=new Map(),ids=points.map(p=>{const key=p.join(',');if(!nodeIds.has(key))nodeIds.set(key,nodeIds.size);return nodeIds.get(key);}),edges=new Map();let volume=0;const reference=points[index[0]],sub=p=>p.map((x,a)=>x-reference[a]);for(let i=0;i<index.length;i+=3){const tri=index.slice(i,i+3),[a,z,c]=tri.map(id=>sub(points[id]));volume+=(a[0]*(z[1]*c[2]-z[2]*c[1])+a[1]*(z[2]*c[0]-z[0]*c[2])+a[2]*(z[0]*c[1]-z[1]*c[0]))/6;for(let j=0;j<3;j++){const a=ids[tri[j]],b=ids[tri[(j+1)%3]],key=[Math.min(a,b),Math.max(a,b)].join(','),e=edges.get(key)??{count:0,balance:0};e.count++;e.balance+=a<b?1:-1;edges.set(key,e);}}for(const e of edges.values()){assert.equal(e.count,2);assert.equal(e.balance,0);}
 const areaXY=L*W*((u1-u0)*(v1-v0)-(holed?(hu1-hu0)*(hv1-hv0):0)),topArea=areaXY*Math.sqrt(1+(b/L)**2+(b/W)**2),surfaceArea=areaXY+topArea+2*(L*(u1-u0)+W*(v1-v0)+(holed?L*(hu1-hu0)+W*(hv1-hv0):0))*(H+Math.max(b,0)/4),worldScale=Math.max(...args.slice(6,9).map(Math.abs),L,W,H+Math.abs(b)),roundoff=4*2**-23*worldScale*Math.sqrt(3),budget=(1e-4*topArea+roundoff*surfaceArea)*scale**3;assert.ok(volume>0);assert.ok(Math.abs(volume-expected*scale**3)<=budget,JSON.stringify({volume,expected:expected*scale**3,budget}));
 const cosine=Math.cos(angle),sine=Math.sin(angle),local=points.map(p=>{const [x,y,z]=p.map((v,i)=>v/scale-[tx,ty,tz][i]);return [cosine*x-sine*z,y,sine*x+cosine*z];}),coordinateGuard=roundoff+1e-5;
 for(const p of local){assert.ok(p[0]>=L*u0-coordinateGuard&&p[0]<=L*u1+coordinateGuard&&p[1]>=W*v0-coordinateGuard&&p[1]<=W*v1+coordinateGuard&&p[2]>=-coordinateGuard&&p[2]<=H+Math.max(b,0)/4+coordinateGuard);}
 const u=holed?(u0+hu0)/2:(u0+u1)/2,v=(v0+v1)/2,hit=crossings(local,index,L*u,W*v);assert.equal(hit.length,2);assert.ok(Math.abs(hit[0])<=coordinateGuard);const height=H+4*b*u*(1-u)*v*(1-v);assert.ok(Math.abs(hit[1]-height)<=1e-4*Math.sqrt(1+(b/L)**2+(b/W)**2)+coordinateGuard);if(holed)assert.deepEqual(crossings(local,index,L*(hu0+hu1)/2,W*(hv0+hv1)/2),[]);
 }
}
console.log('Independent STEP reader: 4 exact NURBS documents, mm/metre conversion, imported face counts, closed triangle incidence, bounded mesh volume, conservative enclosures and material/hole ray crossings passed. External mesh volume is approximate.');
