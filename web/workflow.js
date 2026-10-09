import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try {
 const viewer=createMeshViewer($('view'));
 const response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build the WASM module first.');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;
 let draft={schema_version:1,units:'mm',tolerance:{linear:1e-8,angular:1e-10,relative:1e-12},operations:[{kind:'box',id:'box-1',size:[80,60,24]},{kind:'bore',id:'bore-1',input:'box-1',mode:'blind',center:[0,0],radius:14,depth:16}]};
 let lastValid=null;
 window.haganeWorkflow={ready:false,report:null,lastValid:null};
 function evaluate(text){k.hagane_workflow_begin();for(const byte of new TextEncoder().encode(text)){if(k.hagane_workflow_push_byte(byte))break;}const status=k.hagane_workflow_finish();const result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(result.error??'Model transport failed');return result;}
 function history(document,failed){$('history').replaceChildren();for(const op of document.operations){const item=window.document.createElement('li');item.textContent=`${op.id} · ${op.kind}${op.mode?' / '+op.mode:''}${op.id===failed?' · rejected':''}`;$('history').append(item);}}
 function fill(document){const box=document.operations[0];const bore=document.operations[1];['width','length','height'].forEach((id,i)=>$(id).value=box.size[i]);$('mode').value=bore?.mode??'box_only';if(bore){$('radius').value=bore.radius;$('cx').value=bore.center[0];$('cy').value=bore.center[1];if(bore.depth!=null)$('depth').value=bore.depth;}disable();}
 function disable(){const mode=$('mode').value;for(const id of ['radius','cx','cy'])$(id).disabled=mode==='box_only';$('depth').disabled=mode!=='blind';}
 function rebuild(text,importing=false){$('transport').textContent='';
  const report=evaluate(text);window.haganeWorkflow.report=report;$('save').disabled=!report.ok;
  if(report.ok){draft=report.document;lastValid=report;window.haganeWorkflow.lastValid=report;viewer.setMesh(report.mesh);viewer.setSegments([]);$('result-state').textContent='Current edit · exact B-rep validated';$('diagnostic').textContent='';$('volume').textContent=report.mesh.volume.toLocaleString('en-US',{maximumFractionDigits:1})+' mm³';$('topology').textContent=`${report.mesh.faces} faces / ${report.mesh.edges} edges`;$('document').value=JSON.stringify(draft,null,2);history(draft,null);if(importing)fill(draft);}
  else{const d=report.diagnostic;$('result-state').textContent=lastValid?'Rejected edit · showing previous valid result':'Rejected edit · no valid result';let text=`${d.operation_id??'Document'} · ${d.code}: ${d.message}`;if(d.measured_clearance!=null)text+=` Clearance ${d.measured_clearance.toPrecision(6)} mm; required > ${d.required_clearance.toPrecision(6)} mm.`;if(d.suggestion)text+=' '+d.suggestion;$('diagnostic').textContent=text;viewer.setSegments(report.candidate_segments??[]);history(report.attempted_document??draft,d.operation_id);}
  viewer.render();return report;
 }
 function process(text,importing=false){try{return rebuild(text,importing);}catch(e){$('save').disabled=true;$('result-state').textContent=lastValid?'Rejected edit · showing previous valid result':'Rejected edit · no valid result';$('transport').textContent=e.message;window.haganeWorkflow.report={ok:false,diagnostic:{code:'transport_failed',message:e.message}};return window.haganeWorkflow.report;}}
 function fromControls(){const boxId=draft.operations[0].id;const boreId=draft.operations[1]?.id??'bore-1';const mode=$('mode').value;const document={...draft,operations:[{kind:'box',id:boxId,size:['width','length','height'].map(id=>$(id).valueAsNumber)}]};if(mode!=='box_only'){const bore={kind:'bore',id:boreId,input:boxId,mode,center:[$('cx').valueAsNumber,$('cy').valueAsNumber],radius:$('radius').valueAsNumber};if(mode==='blind')bore.depth=$('depth').valueAsNumber;document.operations.push(bore);}return document;}
 for(const id of ['width','length','height','radius','depth','cx','cy','mode'])$(id).addEventListener(id==='mode'?'change':'input',()=>{disable();const document=fromControls();$('document').value=JSON.stringify(document,null,2);process($('document').value);});
 $('load').addEventListener('click',()=>{try{process($('document').value,true);}catch(e){$('transport').textContent=e.message;}});
 $('save').addEventListener('click',()=>{if(!window.haganeWorkflow.report?.ok)return;const blob=new Blob([JSON.stringify(draft,null,2)],{type:'application/json'});const url=URL.createObjectURL(blob);const a=document.createElement('a');a.href=url;a.download='hagane-model.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);});
 $('file').addEventListener('change',async()=>{const file=$('file').files[0];if(!file)return;if(file.size>65536){$('transport').textContent='Model file exceeds 64 KiB.';return;}try{$('document').value=await file.text();process($('document').value,true);$('transport').textContent='';}catch(e){$('transport').textContent=e.message;}});
 $('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());
 fill(draft);rebuild(JSON.stringify(draft));window.haganeWorkflow.ready=true;
}catch(e){$('result-state').textContent=e.message;console.error(e);}
