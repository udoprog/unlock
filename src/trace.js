(function ($w) {
    const PRECISION = 1000;
    const LIMIT = 0.001;

    let load = () => {
        $w.document.querySelectorAll('.timeline').forEach(($timeline) => {
            let move = null;
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

                if (Math.abs(span.from - span.to) > LIMIT) {
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
                    dataEntries.forEach(el => {
                        el.classList.remove("hidden");
                    });

                    if (slider !== null && $target !== null) {
                        $target.parentElement.removeChild(slider);
                        slider = null;
                    } else {
                        toggle();
                    }
                }

                $w.document.removeEventListener("mousemove", move);
                move = null;
                span.from = 0;
                span.to = 0;
            };

            $target.addEventListener("mousedown", (e) => {
                $w.document.body.classList.add("dragging");

                if (e.button !== 0) {
                    return;
                }

                if (slider !== null && $target !== null) {
                    $target.parentElement.removeChild(slider);
                    slider = null;
                }

                let start = null;

                move = (e) => {
                    let rect = $target.getBoundingClientRect();
                    let current = (e.clientX - rect.left) / rect.width;

                    if (start === null) {
                        start = current;
                    }

                    if (slider === null && Math.abs(start - current) > LIMIT) {
                        slider = $w.document.createElement("div");
                        slider.classList.add("slider");
                        $target.parentElement.insertBefore(slider, $target);
                    }

                    if (slider !== null) {
                        span.from = Math.round(Math.max(Math.min(start, current), 0) * PRECISION) / PRECISION;
                        span.to = Math.round(Math.min(Math.max(start, current), 1) * PRECISION) / PRECISION;

                        slider.style.left = (span.from * 100) + "%";
                        slider.style.width = ((span.to - span.from) * 100) + "%";
                    }
                };

                $w.document.addEventListener("mousemove", move);
            });

            $w.document.addEventListener("mouseup", (e) => {
                $w.document.body.classList.remove("dragging");
                clearSlider();
            });
        });
    };

    $w.addEventListener("load", load);
})(window);
