import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id), maximumBytes=1048576;
try {
 const viewer=createMeshViewer($('view'));const response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build the WASM module first.');const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{});const k=instance.exports;
 let accepted=null,serial=0;window.haganeStep={ready:false,accepted:null,report:null};
 function rejected(error){window.haganeStep.report={ok:false,error:error.message};$('save').disabled=true;$('status').textContent='Import rejected · '+error.message+(accepted?' · showing previous valid solid':'');}
 function importText(text){
  try {
   const bytes=new TextEncoder().encode(text);if(bytes.length>maximumBytes)throw Error('STEP file exceeds 1 MiB.');
   k.hagane_step_import_begin();for(const byte of bytes)if(k.hagane_step_import_push_byte(byte))throw Error('STEP transport rejected.');
   const status=k.hagane_step_import_finish();const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(result.error??'STEP geometry rejected.');
   const size=result.bounds.max.map((v,i)=>v-result.bounds.min[i]), extent=Math.max(...size), center=result.bounds.min.map((v,i)=>v+size[i]/2);
   if(!Number.isFinite(extent)||extent<=0)throw Error('Imported solid cannot be fitted for display.');
   // Fit only display coordinates. The accepted B-rep, metrics and STEP stay in mm.
   const positions=result.mesh.positions.map((v,i)=>(v-center[i%3])*80/extent);if(!positions.every(Number.isFinite))throw Error('Nonfinite display coordinates.');
   viewer.setMesh({positions,normals:result.mesh.normals});accepted=result;window.haganeStep.accepted=result;window.haganeStep.report={ok:true};$('save').disabled=false;$('units').textContent='mm';$('volume').textContent=result.mesh.volume.toLocaleString('en-US',{maximumSignificantDigits:8})+' mm³';$('dimensions').textContent=size.map(v=>v.toLocaleString('en-US',{maximumSignificantDigits:6})).join(' × ')+' mm';$('topology').textContent=`${result.vertices} V / ${result.mesh.edges} E / ${result.mesh.faces} F`;$('status').textContent='Current import · exact B-rep validated';
  }catch(error){rejected(error);}
 }
 async function sample(asset="step-tetrahedron-metres.step"){const ticket=++serial;try{const response=await fetch(asset);if(!response.ok)throw Error('Build the sample asset first.');const text=await response.text();if(ticket!==serial)return;$('document').value=text;importText(text);}catch(error){if(ticket===serial)rejected(error);}}
 $('load').addEventListener('click',()=>{serial++;importText($('document').value);});$('sample').addEventListener('click',()=>sample());$('prism').addEventListener('click',()=>sample('step-prism-example.step'));$('tube').addEventListener('click',()=>sample('step-tube-example.step'));$('bored').addEventListener('click',()=>sample('step-bored-prism-example.step'));
 $('file').addEventListener('change',async()=>{const ticket=++serial,file=$('file').files[0];if(!file)return;try{if(file.size>maximumBytes)throw Error('STEP file exceeds 1 MiB.');const buffer=await file.arrayBuffer();let text;try{text=new TextDecoder('utf-8',{fatal:true}).decode(buffer);}catch{throw Error('STEP file must be UTF-8.');}if(ticket!==serial)return;$('document').value=text;importText(text);}catch(error){if(ticket===serial)rejected(error);}});
 $('save').addEventListener('click',()=>{if(!accepted||!window.haganeStep.report?.ok)return;const url=URL.createObjectURL(new Blob([accepted.step],{type:'application/step'}));const a=document.createElement('a');a.href=url;a.download='hagane-imported.step';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);});
 $('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());await sample();window.haganeStep.ready=true;
}catch(error){$('status').textContent=error.message;console.error(error);}
