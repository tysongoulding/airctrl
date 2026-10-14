import React, { useState, useEffect, useRef, useCallback } from "react";
import { Hand, Camera, CameraOff, Loader2, FlipHorizontal, Zap, Play, Sliders, CheckCircle2 } from "lucide-react";
import { FilesetResolver, HandLandmarker } from "@mediapipe/tasks-vision";
import { SettingsState, DEFAULT_GESTURE_KEYBINDS, GestureKeybindMapping } from "../../types/config";
import { tauriApi } from "../../services/ipc";

interface Landmark {
  x: number;
  y: number;
  z: number;
}

export interface TrackedHandInfo {
  side: "Right Hand" | "Left Hand";
  gestureId: string;
  gesture: string;
  pinchDistance: number;
  activeSector: number;
  pointingAngleDeg: number;
}

interface Props {
  settings?: SettingsState;
  onUpdate?: (partial: Partial<SettingsState>) => void;
  onOpenKeybindsTab?: () => void;
}

// 21 standard MediaPipe Hand landmark bone connections
const HAND_CONNECTIONS = [
  // Thumb
  [0, 1], [1, 2], [2, 3], [3, 4],
  // Index
  [0, 5], [5, 6], [6, 7], [7, 8],
  // Middle
  [0, 9], [9, 10], [10, 11], [11, 12],
  // Ring
  [0, 13], [13, 14], [14, 15], [15, 16],
  // Pinky
  [0, 17], [17, 18], [18, 19], [19, 20],
  // Palm Knuckles
  [5, 9], [9, 13], [13, 17],
];

export const GesturesLiveView: React.FC<Props> = ({ settings, onUpdate, onOpenKeybindsTab }) => {
  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const animFrameRef = useRef<number | null>(null);
  const landmarkerRef = useRef<HandLandmarker | null>(null);

  const [cameraActive, setCameraActive] = useState(false);
  const [cameraError, setCameraError] = useState<string | null>(null);
  const [modelLoading, setModelLoading] = useState(false);
  const [modelReady, setModelReady] = useState(false);
  const [simulationMode, setSimulationMode] = useState(false);

  const [cameraDims, setCameraDims] = useState<{ width: number; height: number }>({ width: 1280, height: 720 });
  const [trackedHands, setTrackedHands] = useState<TrackedHandInfo[]>([]);
  const [measuredFps, setMeasuredFps] = useState<number>(0);
  const [lastTriggeredEvent, setLastTriggeredEvent] = useState<{
    hand: string;
    gesture: string;
    keybind: string;
    action: string;
    timestamp: string;
  } | null>(null);

  const swapHandLabels = settings?.swap_hand_labels ?? false;
  const liveExecution = settings?.live_gesture_execution ?? true;
  const keybinds: GestureKeybindMapping[] = settings?.gesture_keybinds || DEFAULT_GESTURE_KEYBINDS;

  // FPS & Swipe velocity tracking
  const frameCountRef = useRef(0);
  const lastFpsTimeRef = useRef(performance.now());
  const simulationAngleRef = useRef(0);
  const lastWristXRef = useRef<Record<string, { x: number; t: number }>>({});
  const lastFireTimeRef = useRef<number>(0);

  const triggerBoundGesture = useCallback(
    async (gestureId: string, handLabel: string) => {
      const mapping = keybinds.find((k) => k.gestureId === gestureId);
      if (!mapping || !mapping.enabled) return;

      const now = performance.now();
      if (now - lastFireTimeRef.current < 1200) return;
      lastFireTimeRef.current = now;

      setLastTriggeredEvent({
        hand: handLabel,
        gesture: mapping.gesture,
        keybind: mapping.keybind,
        action: mapping.action,
        timestamp: new Date().toLocaleTimeString(),
      });

      try {
        if (mapping.action.toLowerCase() === "radial dial") {
          await tauriApi.toggleRadialDial();
        } else {
          await tauriApi.triggerAction(mapping.action);
        }
      } catch (err) {
        console.warn("Gesture trigger notice:", err);
      }
    },
    [keybinds]
  );

  // Initialize MediaPipe Vision HandLandmarker with 2 Hands tracking
  const initHandLandmarker = async (): Promise<HandLandmarker | null> => {
    if (landmarkerRef.current) return landmarkerRef.current;
    setModelLoading(true);
    try {
      let vision;
      try {
        vision = await FilesetResolver.forVisionTasks("./wasm");
      } catch {
        vision = await FilesetResolver.forVisionTasks(
          "https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.18/wasm"
        );
      }

      let landmarker: HandLandmarker;
      try {
        landmarker = await HandLandmarker.createFromOptions(vision, {
          baseOptions: {
            modelAssetPath: "./models/hand_landmarker.task",
            delegate: "GPU",
          },
          numHands: 2,
          minHandDetectionConfidence: 0.5,
          minHandPresenceConfidence: 0.5,
          minTrackingConfidence: 0.5,
          runningMode: "VIDEO",
        });
      } catch (gpuErr) {
        console.warn("GPU delegate unavailable, attempting CPU delegate:", gpuErr);
        try {
          landmarker = await HandLandmarker.createFromOptions(vision, {
            baseOptions: {
              modelAssetPath: "./models/hand_landmarker.task",
              delegate: "CPU",
            },
            numHands: 2,
            minHandDetectionConfidence: 0.5,
            minHandPresenceConfidence: 0.5,
            minTrackingConfidence: 0.5,
            runningMode: "VIDEO",
          });
        } catch {
          landmarker = await HandLandmarker.createFromOptions(vision, {
            baseOptions: {
              modelAssetPath:
                "https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/1/hand_landmarker.task",
              delegate: "CPU",
            },
            numHands: 2,
            minHandDetectionConfidence: 0.5,
            minHandPresenceConfidence: 0.5,
            minTrackingConfidence: 0.5,
            runningMode: "VIDEO",
          });
        }
      }

      landmarkerRef.current = landmarker;
      setModelReady(true);
      return landmarker;
    } catch (err: any) {
      console.error("Failed to load MediaPipe HandLandmarker:", err);
      setCameraError("MediaPipe AI vision model could not load. Simulation fallback available.");
      return null;
    } finally {
      setModelLoading(false);
    }
  };

  // Start Webcam Stream with high-definition resolution matching display
  const startCamera = async () => {
    setCameraError(null);
    try {
      initHandLandmarker().catch((err) => console.warn("Model pre-init error:", err));

      if (!navigator?.mediaDevices?.getUserMedia) {
        throw new Error("Webcam API (getUserMedia) not supported in this environment");
      }
      const stream = await navigator.mediaDevices.getUserMedia({
        video: {
          width: { ideal: 1280, min: 640 },
          height: { ideal: 720, min: 480 },
          facingMode: "user",
        },
        audio: false,
      });
      streamRef.current = stream;
      if (videoRef.current) {
        videoRef.current.srcObject = stream;
        await videoRef.current.play();

        const vw = videoRef.current.videoWidth || 1280;
        const vh = videoRef.current.videoHeight || 720;
        setCameraDims({ width: vw, height: vh });
        if (canvasRef.current) {
          canvasRef.current.width = vw;
          canvasRef.current.height = vh;
        }
      }
      setCameraActive(true);
      setSimulationMode(false);
    } catch (err: any) {
      console.warn("Webcam access error, falling back to simulation:", err);
      setCameraError(err.message || "Webcam access denied or unavailable");
      setCameraActive(true);
      setSimulationMode(true);
    }
  };

  // Stop Webcam Stream
  const stopCamera = () => {
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((track) => track.stop());
      streamRef.current = null;
    }
    if (videoRef.current) {
      videoRef.current.srcObject = null;
    }
    setCameraActive(false);
    setTrackedHands([]);
    setMeasuredFps(0);
  };

  // Draw 21-landmark skeleton for an individual hand
  const drawHandSkeleton = (
    ctx: CanvasRenderingContext2D,
    landmarks: Landmark[],
    side: "Right Hand" | "Left Hand",
    isPinching: boolean,
    pointingAngleDeg: number,
    gesture: string
  ) => {
    const isRight = side === "Right Hand";
    const boneColor = isRight ? "rgba(216, 121, 67, 0.9)" : "rgba(82, 117, 117, 0.95)";
    const jointColor = isRight ? "#d87943" : "#527575";
    const tipColor = isPinching ? "#10b981" : isRight ? "#d87943" : "#527575";

    // 1. Draw Skeleton Bones
    ctx.lineWidth = 3.5;
    ctx.strokeStyle = boneColor;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    for (const [startIdx, endIdx] of HAND_CONNECTIONS) {
      const p1 = landmarks[startIdx];
      const p2 = landmarks[endIdx];
      ctx.beginPath();
      ctx.moveTo(p1.x, p1.y);
      ctx.lineTo(p2.x, p2.y);
      ctx.stroke();
    }

    // 2. Draw Pointing Vector Ray from Index Tip (8)
    const pIndex = landmarks[8];
    const rayAngleRad = (pointingAngleDeg * Math.PI) / 180;
    ctx.lineWidth = 2.5;
    ctx.strokeStyle = isRight ? "rgba(216, 121, 67, 0.8)" : "rgba(82, 117, 117, 0.8)";
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(pIndex.x, pIndex.y);
    ctx.lineTo(pIndex.x + Math.cos(rayAngleRad) * 90, pIndex.y + Math.sin(rayAngleRad) * 90);
    ctx.stroke();
    ctx.setLineDash([]);

    // 3. Draw 21 Landmark Joint Dots
    for (let i = 0; i < landmarks.length; i++) {
      const p = landmarks[i];
      const isTip = [4, 8, 12, 16, 20].includes(i);
      ctx.beginPath();
      ctx.arc(p.x, p.y, isTip ? 6.5 : 4.5, 0, 2 * Math.PI);
      ctx.fillStyle = isTip ? tipColor : jointColor;
      ctx.fill();
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 1.8;
      ctx.stroke();
    }

    // 4. Draw Floating Hand Side & Gesture Tag near Wrist (0)
    const pWrist = landmarks[0];
    const labelText = `${side.toUpperCase()} • ${gesture}`;
    ctx.font = "bold 11px Ubuntu, sans-serif";
    const textMetrics = ctx.measureText(labelText);
    const tagWidth = textMetrics.width + 16;
    const tagHeight = 22;
    const tagX = Math.max(10, Math.min(ctx.canvas.width - tagWidth - 10, pWrist.x - tagWidth / 2));
    const tagY = Math.min(ctx.canvas.height - 30, pWrist.y + 25);

    ctx.fillStyle = "rgba(18, 17, 19, 0.85)";
    ctx.beginPath();
    ctx.roundRect(tagX, tagY, tagWidth, tagHeight, 6);
    ctx.fill();
    ctx.strokeStyle = isRight ? "rgba(216, 121, 67, 0.6)" : "rgba(82, 117, 117, 0.6)";
    ctx.lineWidth = 1.2;
    ctx.stroke();

    ctx.fillStyle = "#ffffff";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(labelText, tagX + tagWidth / 2, tagY + tagHeight / 2);
  };

  // Render Skeleton & Landmark overlay loop for BOTH hands
  const renderFrame = useCallback(() => {
    const canvas = canvasRef.current;
    const video = videoRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    if (video && video.videoWidth > 0 && video.videoHeight > 0) {
      if (canvas.width !== video.videoWidth || canvas.height !== video.videoHeight) {
        canvas.width = video.videoWidth;
        canvas.height = video.videoHeight;
        setCameraDims({ width: video.videoWidth, height: video.videoHeight });
      }
    }

    frameCountRef.current += 1;
    const now = performance.now();
    const elapsed = now - lastFpsTimeRef.current;
    if (elapsed >= 1000) {
      setMeasuredFps(Math.round((frameCountRef.current * 1000) / elapsed));
      frameCountRef.current = 0;
      lastFpsTimeRef.current = now;
    }

    const width = canvas.width;
    const height = canvas.height;
    ctx.clearRect(0, 0, width, height);

    const handsResults: TrackedHandInfo[] = [];

    // A. Detect real hand landmarks using MediaPipe (up to 2 hands)
    if (
      !simulationMode &&
      landmarkerRef.current &&
      video &&
      video.readyState >= 2 &&
      !video.paused &&
      !video.ended
    ) {
      try {
        const detections = landmarkerRef.current.detectForVideo(video, now);
        if (detections.landmarks && detections.landmarks.length > 0) {
          detections.landmarks.forEach((detectedPts, handIdx) => {
            // Mirror X axis to align directly on mirrored webcam preview
            const handLandmarks: Landmark[] = detectedPts.map((pt) => ({
              x: (1 - pt.x) * width,
              y: pt.y * height,
              z: pt.z || 0,
            }));

            // Opposite hand labeling fix: MediaPipe's categoryName in unflipped video feed
            // is opposite to the user's mirrored camera perspective.
            // Previously `cat === "Left" ? "Right Hand" : "Left Hand"` inverted them.
            // Now `cat === "Right"` maps to `"Right Hand"` and `cat === "Left"` maps to `"Left Hand"`
            // (with optional swapHandLabels override).
            let side: "Right Hand" | "Left Hand" = handIdx === 0 ? "Left Hand" : "Right Hand";
            if (detections.handednesses && detections.handednesses[handIdx] && detections.handednesses[handIdx][0]) {
              const cat = detections.handednesses[handIdx][0].categoryName;
              const isRightPhysical = cat === "Right";
              if (swapHandLabels) {
                side = isRightPhysical ? "Left Hand" : "Right Hand";
              } else {
                side = isRightPhysical ? "Right Hand" : "Left Hand";
              }
            }

            // Pinch Calculation (Thumb Tip 4 to Index Tip 8)
            const pThumb = handLandmarks[4];
            const pIndex = handLandmarks[8];
            const pIndexMcp = handLandmarks[5];
            const pWrist = handLandmarks[0];

            const dx = pThumb.x - pIndex.x;
            const dy = pThumb.y - pIndex.y;
            const pinchDist = Math.hypot(dx, dy) / width;
            const isPinching = pinchDist <= 0.05;

            // Pointing Ray Vector
            const rayDx = pIndex.x - pIndexMcp.x;
            const rayDy = pIndex.y - pIndexMcp.y;
            const angleDeg = ((Math.atan2(rayDy, rayDx) * 180) / Math.PI + 360) % 360;
            const sector = Math.floor(angleDeg / 60) % 6;

            // Horizontal Velocity Swipe Detection
            let swipeDetected: "swipe_left" | "swipe_right" | null = null;
            const prevWrist = lastWristXRef.current[side];
            if (prevWrist && now - prevWrist.t > 40 && now - prevWrist.t < 250) {
              const normVelX = (pWrist.x - prevWrist.x) / width / ((now - prevWrist.t) / 1000);
              if (normVelX <= -1.2) {
                swipeDetected = "swipe_left";
              } else if (normVelX >= 1.2) {
                swipeDetected = "swipe_right";
              }
            }
            lastWristXRef.current[side] = { x: pWrist.x, t: now };

            // Gesture Classification
            let gestureId = "open_palm";
            let gesture = "Open Palm";
            if (swipeDetected === "swipe_left") {
              gestureId = "swipe_left";
              gesture = "Swipe Left";
            } else if (swipeDetected === "swipe_right") {
              gestureId = "swipe_right";
              gesture = "Swipe Right";
            } else if (isPinching) {
              gestureId = "pinch";
              gesture = "Pinch (Click)";
            } else {
              const indexDist = Math.hypot(pIndex.x - pWrist.x, pIndex.y - pWrist.y);
              const middleDist = Math.hypot(handLandmarks[12].x - pWrist.x, handLandmarks[12].y - pWrist.y);
              const ringDist = Math.hypot(handLandmarks[16].x - pWrist.x, handLandmarks[16].y - pWrist.y);
              const pinkyDist = Math.hypot(handLandmarks[20].x - pWrist.x, handLandmarks[20].y - pWrist.y);

              if (indexDist > 90 && middleDist > 90 && ringDist < 75 && pinkyDist < 75) {
                gestureId = "v_sign";
                gesture = "V-Sign (Peace)";
              } else if (indexDist > 90 && middleDist < 80 && ringDist < 70 && pinkyDist < 70) {
                gestureId = "pointing";
                gesture = `Pointing (Sector ${sector})`;
              } else if (indexDist < 80 && middleDist < 80 && ringDist < 70 && pinkyDist < 70) {
                gestureId = "fist";
                gesture = "Fist (Dismiss)";
              }
            }

            if (liveExecution && gestureId !== "open_palm") {
              triggerBoundGesture(gestureId, side);
            }

            handsResults.push({
              side,
              gestureId,
              gesture,
              pinchDistance: pinchDist,
              activeSector: sector,
              pointingAngleDeg: angleDeg,
            });

            drawHandSkeleton(ctx, handLandmarks, side, isPinching, angleDeg, gesture);
          });
        }
      } catch (detErr) {
        console.warn("MediaPipe frame detection error:", detErr);
      }
    }

    // B. Dual-Hand Simulation Fallback (with opposite hand labels corrected)
    if (simulationMode && handsResults.length === 0) {
      simulationAngleRef.current += 0.03;
      const t = simulationAngleRef.current;

      const rightLabel: "Right Hand" | "Left Hand" = swapHandLabels ? "Right Hand" : "Left Hand";
      const leftLabel: "Right Hand" | "Left Hand" = swapHandLabels ? "Left Hand" : "Right Hand";

      // Right side of viewport
      const rCenterX = width * 0.7 + Math.sin(t) * (width * 0.08);
      const rCenterY = height * 0.6 + Math.cos(t * 0.8) * (height * 0.08);
      const rPinchProg = (Math.sin(t * 2) + 1) * 0.5;
      const rPinchDist = 0.03 + rPinchProg * 0.08;

      const rLandmarks: Landmark[] = [
        { x: rCenterX, y: rCenterY + 90, z: 0 },
        { x: rCenterX - 35, y: rCenterY + 60, z: 0 },
        { x: rCenterX - 55, y: rCenterY + 30, z: 0 },
        { x: rCenterX - 70, y: rCenterY, z: 0 },
        { x: rCenterX - 35 + (1 - rPinchProg) * 20, y: rCenterY - 20 + (1 - rPinchProg) * 15, z: 0 },
        { x: rCenterX - 20, y: rCenterY + 10, z: 0 },
        { x: rCenterX - 18, y: rCenterY - 30, z: 0 },
        { x: rCenterX - 15, y: rCenterY - 65, z: 0 },
        { x: rCenterX - 12 + Math.cos(t) * 15, y: rCenterY - 95 + Math.sin(t) * 10, z: 0 },
        { x: rCenterX, y: rCenterY + 5, z: 0 },
        { x: rCenterX + 2, y: rCenterY - 40, z: 0 },
        { x: rCenterX + 5, y: rCenterY - 75, z: 0 },
        { x: rCenterX + 7, y: rCenterY - 105, z: 0 },
        { x: rCenterX + 25, y: rCenterY + 15, z: 0 },
        { x: rCenterX + 28, y: rCenterY - 25, z: 0 },
        { x: rCenterX + 30, y: rCenterY - 55, z: 0 },
        { x: rCenterX + 32, y: rCenterY - 80, z: 0 },
        { x: rCenterX + 50, y: rCenterY + 30, z: 0 },
        { x: rCenterX + 55, y: rCenterY - 5, z: 0 },
        { x: rCenterX + 58, y: rCenterY - 35, z: 0 },
        { x: rCenterX + 60, y: rCenterY - 60, z: 0 },
      ];

      handsResults.push({
        side: rightLabel,
        gestureId: rPinchDist <= 0.05 ? "pinch" : "pointing",
        gesture: rPinchDist <= 0.05 ? "Pinch (Click)" : "Pointing (Sector 0)",
        pinchDistance: rPinchDist,
        activeSector: 0,
        pointingAngleDeg: 45,
      });
      drawHandSkeleton(
        ctx,
        rLandmarks,
        rightLabel,
        rPinchDist <= 0.05,
        45,
        rPinchDist <= 0.05 ? "Pinch (Click)" : "Pointing (Sector 0)"
      );

      // Left side of viewport
      const lCenterX = width * 0.3 + Math.sin(t + Math.PI) * (width * 0.08);
      const lCenterY = height * 0.6 + Math.cos((t + Math.PI) * 0.8) * (height * 0.08);
      const lPinchDist = 0.11;

      const lLandmarks: Landmark[] = [
        { x: lCenterX, y: lCenterY + 90, z: 0 },
        { x: lCenterX + 35, y: lCenterY + 60, z: 0 },
        { x: lCenterX + 55, y: lCenterY + 30, z: 0 },
        { x: lCenterX + 70, y: lCenterY, z: 0 },
        { x: lCenterX + 45, y: lCenterY - 15, z: 0 },
        { x: lCenterX + 20, y: lCenterY + 10, z: 0 },
        { x: lCenterX + 18, y: lCenterY - 30, z: 0 },
        { x: lCenterX + 15, y: lCenterY - 65, z: 0 },
        { x: lCenterX + 12, y: lCenterY - 95, z: 0 },
        { x: lCenterX, y: lCenterY + 5, z: 0 },
        { x: lCenterX - 2, y: lCenterY - 40, z: 0 },
        { x: lCenterX - 5, y: lCenterY - 75, z: 0 },
        { x: lCenterX - 7, y: lCenterY - 105, z: 0 },
        { x: lCenterX - 25, y: lCenterY + 15, z: 0 },
        { x: lCenterX - 28, y: lCenterY - 25, z: 0 },
        { x: lCenterX - 30, y: lCenterY - 55, z: 0 },
        { x: lCenterX - 32, y: lCenterY - 80, z: 0 },
        { x: lCenterX - 50, y: lCenterY + 30, z: 0 },
        { x: lCenterX - 55, y: lCenterY - 5, z: 0 },
        { x: lCenterX - 58, y: lCenterY - 35, z: 0 },
        { x: lCenterX - 60, y: lCenterY - 60, z: 0 },
      ];

      handsResults.push({
        side: leftLabel,
        gestureId: "open_palm",
        gesture: "Open Palm",
        pinchDistance: lPinchDist,
        activeSector: 3,
        pointingAngleDeg: 180,
      });
      drawHandSkeleton(ctx, lLandmarks, leftLabel, false, 180, "Open Palm");
    }

    setTrackedHands(handsResults);

    if (!simulationMode && handsResults.length === 0 && cameraActive) {
      ctx.fillStyle = "rgba(18, 17, 19, 0.65)";
      const promptW = Math.min(380, width * 0.7);
      const promptH = 50;
      const promptX = (width - promptW) / 2;
      const promptY = height * 0.45;
      ctx.beginPath();
      ctx.roundRect(promptX, promptY, promptW, promptH, 8);
      ctx.fill();
      ctx.strokeStyle = "rgba(216, 121, 67, 0.4)";
      ctx.lineWidth = 1.2;
      ctx.stroke();

      ctx.fillStyle = "#ffffff";
      ctx.font = "bold 13px Ubuntu, sans-serif";
      ctx.textAlign = "center";
      ctx.fillText(
        modelLoading ? "Loading MediaPipe AI Hand Model..." : "Show both hands in camera view",
        width * 0.5,
        promptY + 28
      );
    }

    animFrameRef.current = requestAnimationFrame(renderFrame);
  }, [simulationMode, modelLoading, cameraActive, swapHandLabels, liveExecution, triggerBoundGesture]);

  useEffect(() => {
    if (cameraActive) {
      animFrameRef.current = requestAnimationFrame(renderFrame);
    } else {
      if (animFrameRef.current) {
        cancelAnimationFrame(animFrameRef.current);
      }
      const canvas = canvasRef.current;
      if (canvas) {
        const ctx = canvas.getContext("2d");
        ctx?.clearRect(0, 0, canvas.width, canvas.height);
      }
    }
    return () => {
      if (animFrameRef.current) {
        cancelAnimationFrame(animFrameRef.current);
      }
    };
  }, [cameraActive, renderFrame]);

  useEffect(() => {
    return () => {
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((track) => track.stop());
      }
      if (landmarkerRef.current) {
        try {
          landmarkerRef.current.close();
        } catch {
          // ignore
        }
      }
    };
  }, []);

  const rightHand = trackedHands.find((h) => h.side === "Right Hand");
  const leftHand = trackedHands.find((h) => h.side === "Left Hand");

  return (
    <div className="flex flex-col gap-6 w-full">
      {/* Live Video Tracking & Skeleton Viewport */}
      <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-4 shadow-sm w-full">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border pb-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-lg bg-muted border border-border flex items-center justify-center text-foreground">
              <Hand className="w-4 h-4" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
                Dual-Hand Real-Time 21-Landmark Tracking
                {modelReady && (
                  <span className="px-2 py-0.5 text-[10px] rounded bg-emerald-500/10 border border-emerald-500/20 text-emerald-500 font-mono">
                    2 Hands Active
                  </span>
                )}
              </h2>
              <p className="text-xs text-muted-foreground mt-0.5">
                Camera-perspective handedness calibrated • Test hand motions live &amp; trigger custom keybinds
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {/* Flip L/R Hand Labels Button */}
            <button
              onClick={() => onUpdate && onUpdate({ swap_hand_labels: !swapHandLabels })}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-border bg-background hover:bg-muted text-foreground transition-all"
              title="Swap Left/Right hand labels if your camera driver mirrors video in hardware"
            >
              <FlipHorizontal className="w-3.5 h-3.5 text-[#d87943]" />
              <span>{swapHandLabels ? "L/R Swapped" : "Flip L/R Labels"}</span>
            </button>

            {/* Live Gesture Execution Toggle */}
            <button
              onClick={() => onUpdate && onUpdate({ live_gesture_execution: !liveExecution })}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border transition-all ${
                liveExecution
                  ? "bg-emerald-500/15 border-emerald-500/40 text-emerald-600 dark:text-emerald-400 font-semibold"
                  : "bg-background border-border text-muted-foreground hover:text-foreground"
              }`}
              title="Automatically fire bound keybinds/actions when performing gestures on camera"
            >
              <Zap className="w-3.5 h-3.5" />
              <span>{liveExecution ? "Live Keybinds: ON" : "Live Keybinds: OFF"}</span>
            </button>

            {/* Configure Keybinds Shortcut */}
            {onOpenKeybindsTab && (
              <button
                onClick={onOpenKeybindsTab}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-border bg-muted hover:bg-muted/80 text-foreground transition-all"
              >
                <Sliders className="w-3.5 h-3.5" />
                <span>Edit Keybinds</span>
              </button>
            )}

            <button
              onClick={cameraActive ? stopCamera : startCamera}
              className={`flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all shadow-sm ${
                cameraActive
                  ? "bg-rose-500/10 border border-rose-500/30 text-rose-500 hover:bg-rose-500/20"
                  : "bg-primary text-primary-foreground hover:bg-primary/90"
              }`}
            >
              {cameraActive ? <CameraOff className="w-3.5 h-3.5" /> : <Camera className="w-3.5 h-3.5" />}
              {cameraActive ? "Stop Camera" : "Enable Live Tracking"}
            </button>
          </div>
        </div>

        {/* Live Triggered Gesture Banner */}
        {lastTriggeredEvent && (
          <div className="px-4 py-2.5 rounded-xl border border-emerald-500/30 bg-emerald-500/10 flex flex-wrap items-center justify-between gap-2 text-xs font-mono">
            <div className="flex items-center gap-2">
              <CheckCircle2 className="w-4 h-4 text-emerald-500 shrink-0" />
              <span className="text-foreground font-semibold">{lastTriggeredEvent.hand}:</span>
              <span className="text-[#d87943] font-bold">{lastTriggeredEvent.gesture}</span>
              <span className="text-muted-foreground">&rarr;</span>
              <span className="px-2 py-0.5 rounded bg-background border border-border text-foreground font-semibold">
                Keybind: {lastTriggeredEvent.keybind}
              </span>
              <span className="text-emerald-600 dark:text-emerald-400">
                ({lastTriggeredEvent.action})
              </span>
            </div>
            <span className="text-[10px] text-muted-foreground">{lastTriggeredEvent.timestamp}</span>
          </div>
        )}

        {/* Responsive Viewport Display Container matching camera aspect ratio */}
        <div
          className="relative w-full rounded-xl overflow-hidden bg-black border border-border flex items-center justify-center shadow-inner transition-all duration-200"
          style={{
            aspectRatio: `${cameraDims.width} / ${cameraDims.height}`,
            maxHeight: "calc(100vh - 340px)",
          }}
        >
          <video
            ref={videoRef}
            className={`absolute inset-0 w-full h-full object-contain transform -scale-x-100 transition-opacity duration-300 ${
              cameraActive && streamRef.current ? "opacity-90" : "opacity-0"
            }`}
          />
          <canvas
            ref={canvasRef}
            width={cameraDims.width}
            height={cameraDims.height}
            className="absolute inset-0 w-full h-full object-contain pointer-events-none"
          />

          {!cameraActive && (
            <div className="flex flex-col items-center gap-3 text-center p-6 z-10">
              <div className="w-12 h-12 rounded-xl bg-muted border border-border flex items-center justify-center text-foreground shadow-sm">
                <Camera className="w-6 h-6" />
              </div>
              <div>
                <p className="text-sm font-semibold text-white">Dual-Hand Camera Tracking Inactive</p>
                <p className="text-xs text-zinc-400 max-w-md mt-1">
                  Click &quot;Enable Live Tracking&quot; to test your webcam and hand motions (Pinch, Pointing, Fist, V-Sign, Swipe Left/Right) in real time.
                </p>
              </div>
              <button
                onClick={startCamera}
                className="mt-2 px-4 py-2 bg-primary text-primary-foreground hover:bg-primary/90 rounded-lg text-xs font-semibold shadow-md transition-all flex items-center gap-2"
              >
                <Camera className="w-4 h-4" /> Start Live Feed
              </button>
            </div>
          )}

          {/* Top Left: Tracking Engine Status & Live Specs */}
          {cameraActive && (
            <div className="absolute top-4 left-4 flex items-center gap-2.5 bg-background/85 backdrop-blur-md px-3.5 py-2 rounded-lg border border-border text-xs font-mono shadow-md">
              <div
                className={`w-2.5 h-2.5 rounded-full ${
                  simulationMode
                    ? "bg-amber-500 animate-pulse"
                    : trackedHands.length > 0
                    ? "bg-emerald-500 animate-pulse"
                    : "bg-blue-400"
                }`}
              />
              <span className="text-foreground font-semibold">
                {simulationMode
                  ? "Simulation Demo"
                  : trackedHands.length === 2
                  ? "Both Hands Tracked (2/2)"
                  : trackedHands.length === 1
                  ? "1 Hand Tracked (1/2)"
                  : modelLoading
                  ? "Loading Model..."
                  : "Active (Searching...)"}
              </span>
              <span className="text-muted-foreground">|</span>
              <span className="text-foreground font-bold">{measuredFps} FPS</span>
              <span className="text-muted-foreground">|</span>
              <span className="text-muted-foreground">{cameraDims.width}×{cameraDims.height}</span>
            </div>
          )}

          {/* Top Right: Individual Hand Badges */}
          {cameraActive && (
            <div className="absolute top-4 right-4 flex items-center gap-2">
              {modelLoading && (
                <div className="flex items-center gap-1.5 bg-background/85 backdrop-blur-md px-3 py-1.5 rounded-lg border border-border text-xs font-mono">
                  <Loader2 className="w-3.5 h-3.5 animate-spin text-primary" />
                  <span className="text-foreground">Loading Vision Model...</span>
                </div>
              )}

              {/* Left Hand Indicator Badge */}
              <div
                className={`flex items-center gap-2 backdrop-blur-md px-3 py-1.5 rounded-lg border text-xs font-mono transition-all ${
                  leftHand
                    ? "bg-background/90 border-[#527575] text-[#527575] shadow-sm font-semibold"
                    : "bg-background/60 border-border/40 text-muted-foreground opacity-60"
                }`}
              >
                <span>🤚 Left Hand:</span>
                <span className="font-bold">{leftHand ? leftHand.gesture : "Searching"}</span>
              </div>

              {/* Right Hand Indicator Badge */}
              <div
                className={`flex items-center gap-2 backdrop-blur-md px-3 py-1.5 rounded-lg border text-xs font-mono transition-all ${
                  rightHand
                    ? "bg-background/90 border-primary text-primary shadow-sm font-semibold"
                    : "bg-background/60 border-border/40 text-muted-foreground opacity-60"
                }`}
              >
                <span>✋ Right Hand:</span>
                <span className="font-bold">{rightHand ? rightHand.gesture : "Searching"}</span>
              </div>
            </div>
          )}

          {/* Bottom Overlay: Dual Hand Spatial Metrics */}
          {cameraActive && trackedHands.length > 0 && (
            <div className="absolute bottom-4 left-4 right-4 flex items-center justify-between bg-background/85 backdrop-blur-md px-5 py-2.5 rounded-xl border border-border text-xs font-mono shadow-md">
              {/* Left Hand Details */}
              <div className="flex items-center gap-4">
                <span className="text-[#527575] font-bold">LEFT HAND:</span>
                {leftHand ? (
                  <>
                    <div className="flex items-center gap-1.5">
                      <span className="text-muted-foreground">Pinch:</span>
                      <span className={`font-bold ${leftHand.pinchDistance <= 0.05 ? "text-emerald-500" : "text-foreground"}`}>
                        {leftHand.pinchDistance.toFixed(3)}
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5">
                      <span className="text-muted-foreground">Sector:</span>
                      <span className="px-2 py-0.5 rounded bg-muted border border-border text-foreground font-bold">
                        Sector {leftHand.activeSector}
                      </span>
                    </div>
                  </>
                ) : (
                  <span className="text-muted-foreground italic">Hand not in frame</span>
                )}
              </div>

              <div className="h-4 w-[1px] bg-border/60" />

              {/* Right Hand Details */}
              <div className="flex items-center gap-4">
                <span className="text-primary font-bold">RIGHT HAND:</span>
                {rightHand ? (
                  <>
                    <div className="flex items-center gap-1.5">
                      <span className="text-muted-foreground">Pinch:</span>
                      <span className={`font-bold ${rightHand.pinchDistance <= 0.05 ? "text-emerald-500" : "text-foreground"}`}>
                        {rightHand.pinchDistance.toFixed(3)}
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5">
                      <span className="text-muted-foreground">Sector:</span>
                      <span className="px-2 py-0.5 rounded bg-muted border border-border text-foreground font-bold">
                        Sector {rightHand.activeSector}
                      </span>
                    </div>
                  </>
                ) : (
                  <span className="text-muted-foreground italic">Hand not in frame</span>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Interactive Hand Motion & Keybind Test Strip */}
        <div className="p-4 rounded-xl border border-border bg-card flex flex-col gap-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div>
              <h3 className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                <Play className="w-3.5 h-3.5 text-[#d87943]" /> Interactive Hand Motion &amp; Keybind Tester
              </h3>
              <p className="text-[11px] text-muted-foreground">
                Perform these motions in front of the camera or click any gesture below to test its bound keybind/action immediately
              </p>
            </div>
            {onOpenKeybindsTab && (
              <button
                onClick={onOpenKeybindsTab}
                className="text-xs font-mono text-[#d87943] hover:underline flex items-center gap-1"
              >
                Customize Gesture Keybinds &rarr;
              </button>
            )}
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-2">
            {keybinds.map((kb) => (
              <button
                key={kb.gestureId}
                onClick={() => triggerBoundGesture(kb.gestureId, "Manual Test")}
                className="p-2.5 rounded-lg border border-border bg-background hover:border-[#d87943] hover:bg-muted/50 text-left transition-all flex flex-col justify-between gap-1 group"
              >
                <div className="flex items-center justify-between w-full">
                  <span className="text-xs font-semibold text-foreground group-hover:text-[#d87943] truncate">
                    {kb.gesture}
                  </span>
                </div>
                <div className="flex items-center justify-between text-[10px] font-mono text-muted-foreground pt-1 border-t border-border/50">
                  <span className="text-foreground font-semibold">{kb.keybind}</span>
                  <span className="truncate ml-1">({kb.action})</span>
                </div>
              </button>
            ))}
          </div>
        </div>

        {cameraError && (
          <div className="p-3.5 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-600 dark:text-amber-400 text-xs">
            Notice: {cameraError}. Dual-hand simulated loop active for interface demonstration.
          </div>
        )}
      </div>
    </div>
  );
};
