"""CLI command: experiment — Track experiments."""
import argparse
from cli._shared import print_info, print_error
from llm.experiment_tracker import ExperimentTracker

def _build_experiment_parser(subparsers):
    p = subparsers.add_parser("experiment", help="Track experiments")
    sub = p.add_subparsers(dest="action")

    p_list = sub.add_parser("list", help="List experiments")
    p_list.add_argument("--status")
    p_list.add_argument("--milestone", "-m")
    p_list.add_argument("-v", "--verbose", action="store_true")

    p_run = sub.add_parser("run", help="Start experiment")
    p_run.add_argument("name", help="Experiment name")
    p_run.add_argument("--desc", help="Description")
    p_run.add_argument("--milestone", "-m", help="Roadmap milestone ID")
    p_run.add_argument("--tag", nargs="+", help="Tags")

    p_get = sub.add_parser("get", help="Get experiment")
    p_get.add_argument("id", help="Experiment ID")

    p_complete = sub.add_parser("complete", help="Mark completed")
    p_complete.add_argument("id")
    p_complete.add_argument("--metrics", help="JSON metrics")

    p_metric = sub.add_parser("metric", help="Add metric")
    p_metric.add_argument("id")
    p_metric.add_argument("name")
    p_metric.add_argument("value", type=float)
    p_metric.add_argument("--unit", default="", help="Unit (e.g., %, s)")

    p_compare = sub.add_parser("compare", help="Compare experiments")
    p_compare.add_argument("ids", nargs="+", help="Experiment IDs")
    p_compare.add_argument("--metrics", nargs="+", help="Specific metrics")

    p_delete = sub.add_parser("delete", help="Delete experiment")
    p_delete.add_argument("id")

    p_simulate = sub.add_parser("simulate", help="Simulate experiment outcome for testing feedback loop")
    p_simulate.add_argument("id", help="Experiment ID")
    p_simulate.add_argument("result", choices=["success", "fail"], help="Simulated outcome")

    return p

def _run_experiment(args):
    tracker = ExperimentTracker()

    if args.action == "list":
        exps = tracker.list_experiments(status=args.status, milestone=args.milestone)
        print(tracker.render_list(exps, verbose=args.verbose))

    elif args.action == "run":
        e = tracker.run(args.name, description=args.desc or "", roadmap_milestone=args.milestone or "", tags=args.tag or [])
        print(f"⚡ Started experiment [{e.id}]: {e.name}")

    elif args.action == "get":
        e = tracker.get(args.id)
        if e:
            print(f"Experiment: {e.name}")
            print(f"ID: {e.id}")
            print(f"Status: {e.status}")
            print(f"Created: {e.created_at}")
            if e.roadmap_milestone: print(f"Milestone: {e.roadmap_milestone}")
            if e.metrics: print(f"Metrics: " + ", ".join(f"{m.name}={m.value}" for m in e.metrics))
        else:
            print_error(f"Experiment [{args.id}] not found")

    elif args.action == "complete":
        import json
        results = json.loads(args.metrics) if args.metrics else None
        e = tracker.complete(args.id, results)
        if e: print(f"✓ Completed [{e.id}]: {e.name}")
        else: print_error(f"Experiment [{args.id}] not found")

    elif args.action == "metric":
        e = tracker.add_metric(args.id, args.name, args.value, args.unit or "")
        if e: print(f"✓ Added metric {args.name}={args.value}{args.unit or ''} to [{e.id}]")
        else: print_error(f"Experiment [{args.id}] not found")

    elif args.action == "compare":
        comp = tracker.compare(args.ids, args.metrics)
        print(tracker.render_compare(comp))

    elif args.action == "delete":
        if tracker.delete(args.id): print(f"✓ Deleted [{args.id}]")
        else: print_error(f"Experiment [{args.id}] not found")

    elif args.action == "simulate":
        e = tracker.get(args.id)
        if not e:
            print_error(f"Experiment [{args.id}] not found")
            return 1
        if e.status != "running":
            print_error(f"Experiment [{args.id}] is not running (status: {e.status})")
            return 1
        if args.result == "success":
            tracker.complete(args.id, {"simulated": True, "outcome": "success"})
            print(f"✅ Simulated success for [{args.id}]: {e.name}")
        else:
            tracker.fail(args.id, error="simulated failure")
            print(f"❌ Simulated failure for [{args.id}]: {e.name}")
        print_info("  → VALIDATED/REJECTED event written to evolution tracker")

    else:
        print_error("Unknown action")
        return 1
    return 0
