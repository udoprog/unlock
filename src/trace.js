(function($w) {
    let load = () => {
        $w.document.querySelectorAll('.timeline').forEach(($timeline) => {
            let move = null;
            let start = null;
            let slider = null;
            let span = { from: null, to: null };

            let id = $timeline.getAttribute("data-toggle");
            let $details = $w.document.getElementById(id);
            let $target = $timeline.querySelector(".timeline-target");

            if (!$details || !$timeline || !$target) {
                return;
            }

            let show = () => {
                $details.classList.add("visible");
            };

            let toggle = () => {
                $details.classList.toggle("visible");
            };

            let clearSlider = () => {
                if (span.from !== null && span.to !== null) {
                    let start = parseInt($timeline.getAttribute("data-start"));
                    let end = parseInt($timeline.getAttribute("data-end"));
    
                    let duration = end - start;
    
                    let from = duration * span.from;
                    let to = duration * span.to;
    
                    $details.querySelectorAll("[data-entry]").forEach(el => {
                        let entryStart = parseInt(el.getAttribute("data-entry-start"));
                        let entryClose = parseInt(el.getAttribute("data-entry-close"));
    
                        if (entryStart < from || entryClose > to) {
                            el.classList.add("hidden");
                        } else {
                            el.classList.remove("hidden");
                        }
                    });
    
                    show();

                    span.from = null;
                    span.to = null;
                } else {
                    if (slider !== null && $target !== null) {
                        $target.parentElement.removeChild(slider);
                        slider = null;
                    }

                    $details.querySelectorAll("[data-entry]").forEach(el => {
                        el.classList.remove("hidden");
                    });
    
                    toggle();
                }
    
                if (move !== null) {
                    $target.removeEventListener("mousemove", move);
                    move = null;
                }
    
                start = null;
            };

            $target.addEventListener("mousedown", (e) => {
                if (slider !== null && $target !== null) {
                    $target.parentElement.removeChild(slider);
                    slider = null;
                }

                let newSlider = $w.document.createElement("div");
                newSlider.classList.add("slider");
                $target.parentElement.insertBefore(newSlider, $target);

                slider = newSlider;

                start = e.offsetX / $target.clientWidth;

                move = (e) => {
                    let current = e.offsetX / $target.clientWidth;

                    span.from = Math.min(start, current);
                    span.to = Math.max(start, current);

                    newSlider.style.left = Math.round(span.from * 100) + "%";
                    newSlider.style.width = Math.round((span.to - span.from) * 100) + "%";
                };

                $target.addEventListener("mousemove", move);
            });

            $target.addEventListener("mouseup", (e) => {
                clearSlider();
            });
        });
    };

    $w.addEventListener("load", load);
})(window);
