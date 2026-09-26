import { useEffect, useRef, type ReactNode, type RefObject } from "react";
import { shouldCloseDetailPanel } from "./detailPanelState";

type DetailPanelProps = {
  open: boolean;
  id: string;
  eyebrow: string;
  title: string;
  ariaLabel: string;
  onClose: () => void;
  returnFocusRef: RefObject<HTMLElement | null>;
  fallbackFocusRef?: RefObject<HTMLElement | null>;
  children: ReactNode;
  className?: string;
};

export function DetailPanel({open,id,eyebrow,title,ariaLabel,onClose,returnFocusRef,fallbackFocusRef,children,className=""}:DetailPanelProps){
  const panelRef=useRef<HTMLElement>(null);
  const wasOpen=useRef(false);

  useEffect(()=>{
    if(open){
      wasOpen.current=true;
      panelRef.current?.focus({preventScroll:true});
      return;
    }
    if(wasOpen.current){
      wasOpen.current=false;
      const target=returnFocusRef.current?.isConnected?returnFocusRef.current:fallbackFocusRef?.current;
      target?.focus();
    }
  },[open,returnFocusRef,fallbackFocusRef]);

  useEffect(()=>{
    if(!open)return;
    const handleKeyDown=(event:KeyboardEvent)=>{
      if(!shouldCloseDetailPanel(event))return;
      event.preventDefault();
      onClose();
    };
    window.addEventListener("keydown",handleKeyDown);
    return()=>window.removeEventListener("keydown",handleKeyDown);
  },[open,onClose]);

  if(!open)return null;
  const titleId=`${id}-title`;
  return <aside ref={panelRef} id={id} className={`detail-panel ${className}`.trim()} aria-label={ariaLabel} aria-labelledby={titleId} tabIndex={-1}>
    <div className="detail-heading"><div><p className="eyebrow">{eyebrow}</p><h2 id={titleId}>{title}</h2></div><button type="button" className="quiet" onClick={onClose} aria-label={`Close ${ariaLabel}`}>Close</button></div>
    {children}
  </aside>;
}
