import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view'),{doubleSided:true}),response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;
 window.haganeSurfaceHole={ready:false,data:null};
 function generate(){
  const height=Number($('height').value),weight=Number($('weight').value),error=Number($('surface-error').value),width=Number($('hole-width').value);
  const status=k.hagane_generate_surface_hole(height,weight,error,width,Number($('crease').checked)),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  viewer.setMesh(data);const segments=[];for(const section of data.sections)for(let i=3;i<section.samples.length;i+=3)segments.push([section.samples.slice(i-3,i),section.samples.slice(i,i+3)]);viewer.setSegments(segments);window.haganeSurfaceHole.data=data;
  $('height-label').value=height.toFixed(0);$('weight-label').value=weight.toFixed(2);$('surface-error-label').value=error.toFixed(2);$('hole-width-label').value=width.toFixed(2);$('hole-domain').textContent=data.holes[0].map(r=>'['+r.map(x=>x.toFixed(2)).join(', ')+']').join(' × ');$('wire-count').textContent=String(data.brep.wire_edges.length);$('edge-count').textContent=String(data.brep.edge_vertices.length);$('cells').textContent=String(data.cells);$('surface-bound').textContent=Math.max(...data.cell_bounds).toExponential(3);$('status').textContent='✓ Exact inner wire validated · hole excluded from bounded mesh';
 }
 const update=()=>{try{generate();}catch(e){$('status').textContent=e.message;}};for(const id of ['height','weight','surface-error','hole-width'])$(id).addEventListener('input',update);$('crease').addEventListener('change',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeSurfaceHole.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
