import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view')),response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;window.haganeGraphSolid={ready:false,data:null};
 let accepted=null,displayCenter=[0,0,0],boundarySegments=[];
 function generate(){
  const [width,depth,height,bulge,error]=['width','depth','height','bulge','surface-error'].map(id=>Number($(id).value));
  const angle=Number($('angle').value)*Math.PI/180,[tx,ty,tz]=['tx','ty','tz'].map(id=>Number($(id).value));
  const domain=['u-min','u-max','v-min','v-max'].map(id=>Number($(id).value));
  const status=k.hagane_generate_graph_solid_trimmed(width,depth,height,bulge,error,angle,tx,ty,tz,...domain),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  const center=[0,1,2].map(axis=>{let lo=Infinity,hi=-Infinity;for(let i=axis;i<data.positions.length;i+=3){lo=Math.min(lo,data.positions[i]);hi=Math.max(hi,data.positions[i]);}return (lo+hi)/2;});
  const segments=[];for(const samples of data.boundary_samples)for(let i=3;i<samples.length;i+=3)segments.push([samples.slice(i-3,i),samples.slice(i,i+3)]);
  viewer.setMesh({positions:data.positions.map((x,i)=>x-center[i%3]),normals:data.normals});viewer.setSegments(segments.map(segment=>segment.map(point=>point.map((x,i)=>x-center[i]))));window.haganeGraphSolid.data=data;accepted=data;displayCenter=center;boundarySegments=segments;window.haganeGraphSolid.query=null;$('classification').textContent='Model accepted. Classify a world point.';
  $('surface-error-label').value=error.toFixed(3);$('volume').textContent=data.volume.toFixed(3)+' mm³';$('faces').textContent=String(data.brep.faces);$('edges').textContent=String(data.brep.edges);$('triangles').textContent=String(data.mesh.triangles.length);$('surface-bound').textContent=data.error_bounds.reduce((maximum,bound)=>Math.max(maximum,bound),0).toExponential(3);$('status').textContent='✓ Closed graph B-rep validated · exact volume · bounded triangles';
 }
 function showPoint(point){
  if(!point.every((x,i)=>Number.isFinite(x-displayCenter[i])&&Math.abs(x-displayCenter[i])<1e30))return;
  const cross=[];for(let axis=0;axis<3;axis++){const a=[...point],b=[...point];a[axis]-=2.5;b[axis]+=2.5;cross.push([a,b]);}
  viewer.setSegments([...boundarySegments,...cross].map(segment=>segment.map(point=>point.map((x,i)=>x-displayCenter[i]))));
 }
 function classify(){
  if(!accepted)return;const p=['point-x','point-y','point-z'].map(id=>Number($(id).value)),tolerance=Number($('point-tolerance').value),d=accepted,domain=d.source_domain.flat();
  try{const status=k.hagane_classify_graph(d.width,d.depth,d.height,d.bulge,d.error,d.placement.angle,...d.placement.translation,...domain,...p,tolerance),query=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(query.error);window.haganeGraphSolid.query=query;$('classification').textContent=query.location+' · '+query.reason;if(p.every(Number.isFinite))showPoint(p);}
  catch(e){window.haganeGraphSolid.query={error:e.message,point:p};$('classification').textContent='Query rejected: '+e.message;if(p.every(Number.isFinite))showPoint(p);}
 }
 $('classify').addEventListener('click',classify);
 const update=()=>{try{generate();}catch(e){$('status').textContent=e.message;}};$('apply').addEventListener('click',update);$('surface-error').addEventListener('input',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeGraphSolid.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
