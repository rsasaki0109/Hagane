import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view'),{doubleSided:true}),response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;window.haganeSurfacePolygon={ready:false,data:null};
 function generate(){
  const height=Number($('height').value),weight=Number($('weight').value),error=Number($('surface-error').value),mode=Number($('mode').value),corners=['u0','v0','u1','v1','u2','v2'].map(id=>Number($(id).value));
  const holes=['hole-u0','hole-u1','hole-v0','hole-v1'].map(id=>Number($(id).value));
  const status=$('hole').checked?k.hagane_generate_polygon_hole(height,weight,error,mode,...corners,...holes):k.hagane_generate_polygon(height,weight,error,mode,...corners),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  // Center only the display coordinates; retained kernel geometry stays unchanged.
  const center=[0,1,2].map(axis=>{let lo=Infinity,hi=-Infinity;for(let i=axis;i<data.positions.length;i+=3){lo=Math.min(lo,data.positions[i]);hi=Math.max(hi,data.positions[i]);}return (lo+hi)/2;});
  const segments=[];for(const samples of data.boundary_samples)for(let i=3;i<samples.length;i+=3)segments.push([samples.slice(i-3,i),samples.slice(i,i+3)]);
  viewer.setMesh({positions:data.positions.map((x,i)=>x-center[i%3]),normals:data.normals});viewer.setSegments(segments.map(segment=>segment.map(point=>point.map((x,i)=>x-center[i]))));window.haganeSurfacePolygon.data=data;
  $('height-label').value=height.toFixed(0);$('weight-label').value=weight.toFixed(2);$('surface-error-label').value=error.toFixed(3);$('edges').textContent=String(data.brep.edges);$('triangles').textContent=String(data.mesh.triangles.length);$('nodes').textContent=String(new Set(data.vertex_nodes).size);$('surface-bound').textContent=Math.max(...data.error_bounds).toExponential(3);$('status').textContent=$('hole').checked?'✓ Exact convex B-rep face validated · rectangular inner wire · bounded surface triangles':'✓ Exact convex B-rep face validated · bounded surface triangles';
 }
 const update=()=>{try{generate();}catch(e){$('status').textContent=e.message;}};$('apply').addEventListener('click',update);for(const id of ['height','weight','surface-error'])$(id).addEventListener('input',update);$('mode').addEventListener('change',update);$('hole').addEventListener('change',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeSurfacePolygon.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
