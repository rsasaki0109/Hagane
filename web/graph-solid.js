import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view')),response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;window.haganeGraphSolid={ready:false,data:null};
 function generate(){
  const [width,depth,height,bulge,error]=['width','depth','height','bulge','surface-error'].map(id=>Number($(id).value));
  const status=k.hagane_generate_graph_solid(width,depth,height,bulge,error),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  const center=[0,1,2].map(axis=>{let lo=Infinity,hi=-Infinity;for(let i=axis;i<data.positions.length;i+=3){lo=Math.min(lo,data.positions[i]);hi=Math.max(hi,data.positions[i]);}return (lo+hi)/2;});
  const segments=[];for(const samples of data.boundary_samples)for(let i=3;i<samples.length;i+=3)segments.push([samples.slice(i-3,i),samples.slice(i,i+3)]);
  viewer.setMesh({positions:data.positions.map((x,i)=>x-center[i%3]),normals:data.normals});viewer.setSegments(segments.map(segment=>segment.map(point=>point.map((x,i)=>x-center[i]))));window.haganeGraphSolid.data=data;
  $('surface-error-label').value=error.toFixed(3);$('volume').textContent=data.volume.toFixed(3)+' mm³';$('faces').textContent=String(data.brep.faces);$('edges').textContent=String(data.brep.edges);$('triangles').textContent=String(data.mesh.triangles.length);$('surface-bound').textContent=data.error_bounds.reduce((maximum,bound)=>Math.max(maximum,bound),0).toExponential(3);$('status').textContent='✓ Closed graph B-rep validated · exact volume · bounded triangles';
 }
 const update=()=>{try{generate();}catch(e){$('status').textContent=e.message;}};$('apply').addEventListener('click',update);$('surface-error').addEventListener('input',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeGraphSolid.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
