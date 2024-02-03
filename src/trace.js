(function ($w) {
    let load = () => {
        $w.document.querySelectorAll('.timeline').forEach(($timeline) => {
            let move = null;
            let start = null;
            let slider = null;
            let span = { from: 0, to: 0 };

            let id = $timeline.getAttribute("data-toggle");
            let $details = $w.document.getElementById(id);
            let $target = $timeline.querySelector(".timeline-target");

            if (!$details || !$timeline || !$target) {
                return;
            }

            let dataEntries = $details.querySelectorAll("[data-entry]");

            let show = () => {
                $details.classList.add("visible");
            };

            let toggle = () => {
                $details.classList.toggle("visible");
            };

            let clearSlider = () => {
                if (move === null) {
                    return;
                }

                if (Math.abs(span.from - span.to) > 0.001) {
                    let start = parseInt($timeline.getAttribute("data-start"));
                    let end = parseInt($timeline.getAttribute("data-end"));
                    let duration = end - start;

                    let from = duration * span.from;
                    let to = duration * span.to;

                    dataEntries.forEach(el => {
                        let entryStart = parseInt(el.getAttribute("data-entry-start"));
                        let entryClose = parseInt(el.getAttribute("data-entry-close"));

                        if (entryStart < from || entryClose > to) {
                            el.classList.add("hidden");
                        } else {
                            el.classList.remove("hidden");
                        }
                    });

                    show();
                } else {
                    if (slider !== null && $target !== null) {
                        $target.parentElement.removeChild(slider);
                        slider = null;
                    }

                    dataEntries.forEach(el => {
                        el.classList.remove("hidden");
                    });

                    toggle();
                }

                $w.document.removeEventListener("mousemove", move);
                move = null;
                start = null;
                span.from = 0;
                span.to = 0;
            };

            $target.addEventListener("mousedown", (e) => {
                if (e.button !== 0) {
                    return;
                }

                if (slider !== null && $target !== null) {
                    $target.parentElement.removeChild(slider);
                    slider = null;
                }

                let newSlider = $w.document.createElement("div");
                newSlider.classList.add("slider");
                $target.parentElement.insertBefore(newSlider, $target);

                slider = newSlider;

                move = (e) => {
                    let rect = $target.getBoundingClientRect();
                    let current = (e.clientX - rect.left) / rect.width;

                    if (start === null) {
                        start = current;
                    }

                    span.from = Math.round(Math.min(start, current) * 1000) / 1000;
                    span.to = Math.round(Math.max(start, current) * 1000) / 1000;

                    newSlider.style.left = (span.from * 100) + "%";
                    newSlider.style.width = ((span.to - span.from) * 100) + "%";
                };

                $w.document.addEventListener("mousemove", move);
            });

            $w.document.addEventListener("mouseup", (e) => {
                clearSlider();
            });
        });
    };

    $w.addEventListener("load", load);
})(window);
